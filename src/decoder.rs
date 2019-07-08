#![allow(dead_code)]
use crate::ebml;
use crate::vint::{read_vint, UnrepresentableLengthError};
use chrono::{DateTime, NaiveDateTime, Utc};
use err_derive::Error;

pub trait ReadEbmlExt: std::io::Read {
    fn read_ebml_to_end(
        &mut self,
        schema: impl ebml::SchemaDict,
    ) -> Result<Vec<ebml::Element>, DecodeError> {
        let mut decoder = Decoder::new(schema);
        let mut buf = vec![];
        let _size = self.read_to_end(&mut buf).map_err(DecodeError::Io)?;
        let elms = decoder.decode(buf)?;
        Ok(elms)
    }
}

impl<R: std::io::Read + ?Sized> ReadEbmlExt for R {}

pub trait BufReadEbmlExt: std::io::BufRead {
    fn read(&mut self, schema: impl ebml::SchemaDict) -> Result<Vec<ebml::Element>, DecodeError> {
        let mut decoder = Decoder::new(schema);
        let mut buf = vec![];
        loop {
            let used = {
                let available = match self.fill_buf() {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(DecodeError::Io(e)),
                };
                buf.append(&mut decoder.decode(available.to_vec())?);
                available.len()
            };
            self.consume(used);
            if used == 0 {
                break;
            }
        }
        Ok(buf)
    }
}

impl<R: std::io::BufRead + ?Sized> BufReadEbmlExt for R {}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error(display = "{}", _0)]
    ReadVint(#[error(cause)] UnrepresentableLengthError),
    #[error(display = "UnknwonSizeNotAllowedInChildElement: detail {:?}", _0)]
    UnknwonSizeNotAllowedInChildElement(ebml::ElementDetail),
    #[error(display = "ReadContent")]
    ReadContent(#[error(cause)] ReadContentError),
    #[error(display = "UnknownSchema: {:?}", _0)]
    UnknownSchema(ebml::EbmlId),
    #[error(display = "Io")]
    Io(#[error(cause)] std::io::Error),
}

impl From<UnrepresentableLengthError> for DecodeError {
    fn from(o: UnrepresentableLengthError) -> Self {
        DecodeError::ReadVint(o)
    }
}

impl From<ReadContentError> for DecodeError {
    fn from(o: ReadContentError) -> Self {
        DecodeError::ReadContent(o)
    }
}

enum State {
    Tag,
    Size,
    Content,
}

pub struct Decoder<D> {
    schema: D,
    state: State,
    buffer: Vec<u8>,
    cursor: usize,
    total: usize,
    stack: Vec<ebml::ElementDetail>,
    queue: Vec<ebml::Element>,
}

impl<D: ebml::SchemaDict> Decoder<D> {
    pub fn new(schema: D) -> Self {
        Self {
            schema,
            state: State::Tag,
            buffer: vec![],
            cursor: 0,
            total: 0,
            stack: vec![],
            queue: vec![],
        }
    }
    pub fn decode(&mut self, chunk: Vec<u8>) -> Result<Vec<ebml::Element>, DecodeError> {
        self.read_chunk(chunk)?;
        let mut result = vec![];
        std::mem::swap(&mut self.queue, &mut result);
        Ok(result)
    }
    fn read_chunk(&mut self, mut chunk: Vec<u8>) -> Result<(), DecodeError> {
        // 読みかけの(読めなかった) buffer と 新しい chunk を合わせて読み直す
        self.buffer.append(&mut chunk);
        while self.cursor < self.buffer.len() {
            match self.state {
                State::Tag => {
                    if !self.read_tag()? {
                        break;
                    }
                }
                State::Size => {
                    if !self.read_size()? {
                        break;
                    }
                }
                State::Content => {
                    if !self.read_content()? {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
    /// return false when waiting for more data
    fn read_tag(&mut self) -> Result<bool, DecodeError> {
        // tag is out of buffer
        if self.cursor >= self.buffer.len() {
            return Ok(false);
        }
        // read ebml id vint without first byte
        let opt_tag_size = read_vint(&self.buffer, self.cursor)?;
        // cannot read tag yet
        if opt_tag_size.is_none() {
            return Ok(false);
        }
        let tag_size = opt_tag_size.unwrap().length;

        let tag_view = &self.buffer[self.cursor..(self.cursor + tag_size as usize)];
        // assert_eq!(tag_view.len(), tag_size as usize);
        let ebml_id = ebml::EbmlId(tag_view.iter().enumerate().fold(0_i64, |o, (v, i)| {
            o + (v as i64) * i64::pow(16_i64, 2_u32 * (u32::from(tag_size) - 1 - u32::from(*i)))
        }));

        let tag_start = self.total;
        let size_start = self.total + (tag_size as usize);
        let content_start = 0;
        let content_size = 0;
        let schema = self
            .schema
            .get(ebml_id)
            .ok_or_else(|| DecodeError::UnknownSchema(ebml_id))?;
        let detail = ebml::ElementDetail {
            level: schema.level,
            r#type: schema.r#type,
            ebml_id,
            tag_start,
            size_start,
            content_start,
            content_size,
        };
        self.stack.push(detail);

        // move cursor
        self.cursor += tag_size as usize;
        self.total += tag_size as usize;

        // change decoder state
        self.state = State::Size;
        Ok(true)
    }
    /// return false when waiting for more data
    fn read_size(&mut self) -> Result<bool, DecodeError> {
        if self.cursor >= self.buffer.len() {
            return Ok(false);
        }

        // read ebml datasize vint without first byte
        let opt_size = read_vint(&self.buffer, self.cursor)?;

        if opt_size.is_none() {
            return Ok(false);
        }
        let size = opt_size.unwrap();

        // decide current tag data size
        let ebml::ElementDetail {
            ref mut tag_start,
            ref mut content_start,
            ref mut content_size,
            ..
        } = self.stack.last_mut().unwrap();
        *content_start = *tag_start + (size.length as usize);
        *content_size = size.value;

        // move cursor and change state
        self.cursor += size.length as usize;
        self.total += size.length as usize;
        self.state = State::Content;

        Ok(true)
    }
    fn read_content(&mut self) -> Result<bool, DecodeError> {
        let current_detail = self.stack.last().unwrap();
        // master element は子要素を持つので生データはない
        if current_detail.r#type == 'm' {
            let elm = ebml::Element::MasterElement(
                ebml::MasterElement::MasterStartElement {
                    ebml_id: current_detail.ebml_id,
                    unknown_size: current_detail.content_size == -1,
                },
                *current_detail,
            );
            self.queue.push(elm);
            self.state = State::Tag;
            // この Mastert Element は空要素か
            if current_detail.content_size == 0 {
                // 即座に終了タグを追加
                self.queue.push(ebml::Element::MasterElement(
                    ebml::MasterElement::MasterEndElement {
                        ebml_id: current_detail.ebml_id,
                    },
                    *current_detail,
                ));
                // スタックからこのタグを捨てる
                self.stack.pop();
            }
            return Ok(true);
        }
        // endless master element
        // waiting for more data
        if current_detail.content_size < 0 {
            return Err(DecodeError::UnknwonSizeNotAllowedInChildElement(
                *current_detail,
            ));
        }
        use std::convert::TryFrom as _;
        let content_size = usize::try_from(current_detail.content_size).unwrap();
        if self.buffer.len() < self.cursor + content_size {
            return Ok(false);
        }
        // タグの中身の生データ
        let content = self.buffer[self.cursor..self.cursor + content_size].to_vec();
        // 読み終わったバッファを捨てて読み込んでいる部分のバッファのみ残す
        self.buffer = self.buffer.split_off(self.cursor + content_size);

        let child_elm = read_child_element(
            current_detail.ebml_id,
            current_detail.r#type,
            std::io::Cursor::new(content),
            content_size,
        )?;
        self.queue
            .push(ebml::Element::ChildElement(child_elm, *current_detail));

        // ポインタを進める
        self.total += content_size;
        // タグ待ちモードに変更
        self.state = State::Tag;
        self.cursor = 0;
        // remove the object from the stack
        self.stack.pop();

        while !self.stack.is_empty() {
            let parent_detail = self.stack.last().unwrap();
            // 親が不定長サイズなので閉じタグは期待できない
            if parent_detail.content_size < 0 {
                self.stack.pop(); // 親タグを捨てる
                return Ok(true);
            }
            // 閉じタグの来るべき場所まで来たかどうか
            if self.total < parent_detail.content_start + content_size {
                break;
            }
            // 閉じタグを挿入すべきタイミングが来た
            if parent_detail.r#type != 'm' {
                // throw new Error("parent element is not master element");
                unreachable!();
            }
            self.queue.push(ebml::Element::MasterElement(
                ebml::MasterElement::MasterEndElement {
                    ebml_id: parent_detail.ebml_id,
                },
                *parent_detail,
            ));
            // スタックからこのタグを捨てる
            self.stack.pop();
        }
        Ok(true)
    }
}

#[derive(Debug, Error)]
pub enum ReadContentError {
    #[error(display = "Date")]
    Date(#[error(cause)] std::io::Error),
    #[error(display = "Utf8")]
    Utf8(#[error(cause)] std::io::Error),
    #[error(display = "UnsignedInteger")]
    UnsignedInteger(#[error(cause)] std::io::Error),
    #[error(display = "Integer")]
    Integer(#[error(cause)] std::io::Error),
    #[error(display = "Float")]
    Float(#[error(cause)] std::io::Error),
    #[error(display = "Binary")]
    Binary(#[error(cause)] std::io::Error),
    #[error(display = "String")]
    String(#[error(cause)] std::io::Error),
    #[error(display = "Master")]
    Master(#[error(cause)] std::io::Error),
    #[error(display = "Unknown")]
    Unknown(#[error(cause)] std::io::Error),
}

fn read_child_element<C: std::io::Read>(
    ebml_id: ebml::EbmlId,
    r#type: char,
    mut content: C,
    content_size: usize,
) -> Result<ebml::ChildElement, ReadContentError> {
    use crate::ebml::ChildElement::*;
    use byteorder::{BigEndian, ReadBytesExt as _};
    use ReadContentError::{String as StringE, *};
    match r#type {
        // Unsigned Integer - Big-endian, any size from 1 to 8 octets
        'u' => {
            let value = content
                .read_uint::<BigEndian>(content_size)
                .map_err(UnsignedInteger)?;
            Ok(UnsignedIntegerElement { ebml_id, value })
        }
        // Signed Integer - Big-endian, any size from 1 to 8 octets
        'i' => {
            let value = content
                .read_int::<BigEndian>(content_size)
                .map_err(Integer)?;
            Ok(IntegerElement { ebml_id, value })
        }
        // Float - Big-endian, defined for 4 and 8 octets (32, 64 bits)
        'f' => {
            let value = if content_size == 4 {
                f64::from(content.read_f32::<BigEndian>().map_err(Float)?)
            } else if content_size == 8 {
                content.read_f64::<BigEndian>().map_err(Float)?
            } else {
                Err(Float(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("invalid float content_size: {}", content_size),
                )))?
            };
            Ok(FloatElement { ebml_id, value })
        }
        //  Printable ASCII (0x20 to 0x7E), zero-padded when needed
        's' => {
            let mut value = vec![0; content_size];
            content.read_exact(&mut value).map_err(StringE)?;
            Ok(StringElement { ebml_id, value })
        }
        //  Unicode string, zero padded when needed (RFC 2279)
        '8' => {
            let mut value = std::string::String::new();
            content.read_to_string(&mut value).map_err(Utf8)?;
            Ok(Utf8Element { ebml_id, value })
        }
        // Binary - not interpreted by the parser
        'b' => {
            let mut value = vec![0; content_size];
            content.read_exact(&mut value).map_err(Binary)?;
            Ok(BinaryElement { ebml_id, value })
        }
        // nano second; Date.UTC(2001,1,1,0,0,0,0) === 980985600000
        // new Date("2001-01-01T00:00:00.000Z").getTime() = 978307200000
        // Date - signed 8 octets integer in nanoseconds with 0 indicating
        // the precise beginning of the millennium (at 2001-01-01T00:00:00,000000000 UTC)
        'd' => {
            use std::convert::TryFrom;
            let nanos = content.read_i64::<BigEndian>().map_err(Date)?;
            let unix_time_nanos: i64 = nanos - 978_307_200 * 1000 * 1000 * 1000;
            let unix_time_secs: i64 = unix_time_nanos / 1000 / 1000 / 1000 - 1;
            let nsecs: u32 =
                u32::try_from((unix_time_nanos & (1000 * 1000 * 1000)) + (1000 * 1000 * 1000))
                    .unwrap();
            let datetime = NaiveDateTime::from_timestamp(unix_time_secs, nsecs);
            let value = DateTime::from_utc(datetime, Utc);
            Ok(DateElement { ebml_id, value })
        }
        // Master-Element - contains other EBML sub-elements of the next lower level
        'm' => Err(Master(std::io::Error::new(
            std::io::ErrorKind::Other,
            "cannot read master element as child element".to_string(),
        )))?,
        _ => Err(Unknown(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("unknown type: {}", r#type),
        )))?,
    }
}

#[cfg(test)]
mod test {}
