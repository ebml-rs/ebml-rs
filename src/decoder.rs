#![allow(dead_code)]
#![allow(unused_imports)]
use crate::ebml;
use crate::vint::{read_vint, ReadVintError};
use err_derive::Error;
use log::trace;

#[derive(Debug, Error)]
enum DecodeError {
    #[error(display = "{}", _0)]
    ReadVintError(#[error(cause)] ReadVintError),
    #[error(display = "{}", _0)]
    TryFromIntError(#[error(cause)] std::num::TryFromIntError),
    #[error(display = "{}", _0)]
    ReadDateError(#[error(cause)] std::io::Error),
    #[error(display = "{}", _0)]
    ReadUtf8Error(#[error(cause)] std::io::Error),
    #[error(display = "{}", _0)]
    ReadDUnsignedIntegerError(#[error(cause)] std::io::Error),
    #[error(display = "{}", _0)]
    ReadIntegerError(#[error(cause)] std::io::Error),
    #[error(display = "{}", _0)]
    ReadFloatError(#[error(cause)] std::io::Error),
}

impl From<ReadVintError> for DecodeError {
    fn from(o: ReadVintError) -> Self {
        DecodeError::ReadVintError(o)
    }
}

impl From<std::num::TryFromIntError> for DecodeError {
    fn from(o: std::num::TryFromIntError) -> Self {
        DecodeError::TryFromIntError(o)
    }
}

enum State {
    Tag,
    Size,
    Content,
}

pub struct Decoder<'a> {
    schema: &'a std::collections::HashMap<i64, ebml::Schema>,
    state: State,
    buffer: Vec<u8>,
    cursor: usize,
    total: usize,
    detail_stack: Vec<ebml::Detail>,
    elm_queue: Vec<ebml::Element>,
}

impl<'a> Decoder<'a> {
    fn new(schema: &'a std::collections::HashMap<i64, ebml::Schema>) -> Self {
        Self {
            schema,
            state: State::Tag,
            buffer: vec![],
            cursor: 0,
            total: 0,
            detail_stack: vec![],
            elm_queue: vec![],
        }
    }
    fn decode(&mut self, chunk: Vec<u8>) -> Result<Vec<ebml::Element>, DecodeError> {
        self.read_chunk(chunk)?;
        let mut result = vec![];
        std::mem::swap(&mut self.elm_queue, &mut result);
        Ok(result)
    }
    fn read_chunk(&mut self, mut chunk: Vec<u8>) -> Result<(), DecodeError> {
        // 読みかけの(読めなかった) buffer と 新しい chunk を合わせて読み直す
        self.buffer.append(&mut chunk);
        while self.cursor < self.buffer.len() {
            trace!("cursor: {}, total: {}", self.cursor, self.total);
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
        assert_eq!(tag_view.len(), tag_size as usize);
        let ebml_id = tag_view.iter().enumerate().fold(0_i64, |o, (v, i)| {
            o + (v as i64) * i64::pow(16_i64, 2_u32 * (u32::from(tag_size) - 1 - u32::from(*i)))
        });

        let tag_start = self.total;
        let size_start = self.total + (tag_size as usize);
        let content_start = 0;
        let content_size = -1;
        let schema = self.get_schema_info(ebml_id)?;
        let detail = ebml::Detail {
            level: schema.level,
            r#type: schema.r#type,
            ebml_id,
            tag_start,
            size_start,
            content_start,
            content_size,
        };
        self.detail_stack.push(detail);

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
        let ebml::Detail {
            ref mut tag_start,
            ref mut content_start,
            ref mut content_size,
            ..
        } = self.detail_stack.last_mut().unwrap();
        *content_start = *tag_start + (size.length as usize);
        *content_size = size.value;

        // move cursor and change state
        self.cursor += size.length as usize;
        self.total += size.length as usize;
        self.state = State::Content;

        Ok(true)
    }
    fn read_content(&mut self) -> Result<bool, DecodeError> {
        let latest_detail = self.detail_stack.last().unwrap();
        // master element は子要素を持つので生データはない
        if latest_detail.r#type == 'm' {
            let elm = ebml::Element::MasterElement(
                ebml::MasterElement::MasterStartElement {
                    ebml_id: latest_detail.ebml_id,
                    unknown_size: latest_detail.content_size == -1,
                },
                *latest_detail,
            );
            self.elm_queue.push(elm);
            self.state = State::Tag;
            // この Mastert Element は空要素か
            if latest_detail.content_size == 0 {
                // 即座に終了タグを追加
                self.elm_queue.push(ebml::Element::MasterElement(
                    ebml::MasterElement::MasterEndElement {
                        ebml_id: latest_detail.ebml_id,
                    },
                    *latest_detail,
                ));
                // スタックからこのタグを捨てる
                self.detail_stack.pop();
            }
            return Ok(true);
        }
        // endless master element
        // waiting for more data
        // if latest_detail.content_size < 0 { return Err(DecodeError::UnknwonSizeNotAllowedInChildElement(latest_detail)); }
        use std::convert::TryFrom as _;
        let content_size = usize::try_from(latest_detail.content_size)?;
        if self.buffer.len() < self.cursor + content_size {
            return Ok(false);
        }
        // タグの中身の生データ
        let content = self.buffer[self.cursor..self.cursor + content_size].to_vec();
        // 読み終わったバッファを捨てて読み込んでいる部分のバッファのみ残す
        self.buffer = self.buffer.split_off(self.cursor + content_size);

        let child_elm: ebml::ChildElement = match latest_detail.r#type {
            // Unsigned Integer - Big-endian, any size from 1 to 8 octets
            'u' => {
                use byteorder::{BigEndian, ReadBytesExt as _};
                let value = std::io::Cursor::new(content)
                    .read_uint::<BigEndian>(content_size)
                    .map_err(DecodeError::ReadDUnsignedIntegerError)?;
                ebml::ChildElement::UnsignedIntegerElement {
                    ebml_id: latest_detail.ebml_id,
                    value,
                }
            }
            // Signed Integer - Big-endian, any size from 1 to 8 octets
            'i' => {
                use byteorder::{BigEndian, ReadBytesExt as _};
                let value = std::io::Cursor::new(content)
                    .read_int::<BigEndian>(content_size)
                    .map_err(DecodeError::ReadIntegerError)?;
                ebml::ChildElement::IntegerElement {
                    ebml_id: latest_detail.ebml_id,
                    value,
                }
            }
            // Float - Big-endian, defined for 4 and 8 octets (32, 64 bits)
            'f' => {
                use byteorder::{BigEndian, ReadBytesExt as _};
                let value = if content_size == 4 {
                    f64::from(
                        std::io::Cursor::new(content)
                            .read_f32::<BigEndian>()
                            .map_err(DecodeError::ReadFloatError)?,
                    )
                } else if content_size == 8 {
                    std::io::Cursor::new(content)
                        .read_f64::<BigEndian>()
                        .map_err(DecodeError::ReadFloatError)?
                } else {
                    Err(DecodeError::ReadFloatError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("invalid float content_size: {}", content_size),
                    )))?
                };
                ebml::ChildElement::FloatElement {
                    ebml_id: latest_detail.ebml_id,
                    value,
                }
            }
            //  Printable ASCII (0x20 to 0x7E), zero-padded when needed
            's' => ebml::ChildElement::StringElement {
                ebml_id: latest_detail.ebml_id,
                value: content,
            },
            //  Unicode string, zero padded when needed (RFC 2279)
            '8' => {
                use std::io::Read;
                let mut value = String::new();
                std::io::Cursor::new(content)
                    .read_to_string(&mut value)
                    .map_err(DecodeError::ReadUtf8Error)?;
                ebml::ChildElement::Utf8Element {
                    ebml_id: latest_detail.ebml_id,
                    value,
                }
            }
            // Binary - not interpreted by the parser
            'b' => ebml::ChildElement::BinaryElement {
                ebml_id: latest_detail.ebml_id,
                value: content,
            },
            // nano second; Date.UTC(2001,1,1,0,0,0,0) === 980985600000
            // Date - signed 8 octets integer in nanoseconds with 0 indicating
            // the precise beginning of the millennium (at 2001-01-01T00:00:00,000000000 UTC)
            'd' => {
                use byteorder::{BigEndian, ReadBytesExt as _};
                let value = std::io::Cursor::new(content)
                    .read_i64::<BigEndian>()
                    .map_err(DecodeError::ReadDateError)?;
                ebml::ChildElement::DateElement {
                    ebml_id: latest_detail.ebml_id,
                    value,
                }
            }
            // Master-Element - contains other EBML sub-elements of the next lower level
            'm' => unreachable!(),
            _ => unreachable!(),
        };
        self.elm_queue
            .push(ebml::Element::ChildElement(child_elm, *latest_detail));

        // ポインタを進める
        self.total += content_size;
        // タグ待ちモードに変更
        self.state = State::Tag;
        self.cursor = 0;
        // remove the object from the stack
        self.detail_stack.pop();

        while !self.detail_stack.is_empty() {
            let parent_detail = self.detail_stack.last().unwrap();
            // 親が不定長サイズなので閉じタグは期待できない
            if parent_detail.content_size < 0 {
                self.detail_stack.pop(); // 親タグを捨てる
                return Ok(true);
            }
            // 閉じタグの来るべき場所まで来たかどうか
            if self.total
                < parent_detail.content_start + usize::try_from(parent_detail.content_size)?
            {
                break;
            }
            // 閉じタグを挿入すべきタイミングが来た
            if parent_detail.r#type != 'm' {
                // throw new Error("parent element is not master element");
                unreachable!();
            }
            self.elm_queue.push(ebml::Element::MasterElement(
                ebml::MasterElement::MasterEndElement {
                    ebml_id: parent_detail.ebml_id,
                },
                *parent_detail,
            ));
            // スタックからこのタグを捨てる
            self.detail_stack.pop();
        }
        Ok(true)
    }
    fn get_schema_info(&self, _ebml_id: i64) -> Result<&'static ebml::Schema, DecodeError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const WEBM_FILE_LIST: &'static [&'static str] = &[
        "../matroska-test-files/test_files/test1.mkv",
        "../matroska-test-files/test_files/test2.mkv",
        "../matroska-test-files/test_files/test3.mkv",
        // "../matroska-test-files/test_files/test4.mkv", this file is broken so not pass encoder_decoder_test
        "../matroska-test-files/test_files/test5.mkv",
        "../matroska-test-files/test_files/test6.mkv",
        // "../matroska-test-files/test_files/test7.mkv", this file has unknown tag so cannot write file
        "../matroska-test-files/test_files/test8.mkv",
    ];
    #[test]
    fn test_decoder() {
        let ebml_schema = std::collections::HashMap::<i64, ebml::Schema>::new();
        let path = WEBM_FILE_LIST[0];
        let mut mkv = std::fs::File::open(path).unwrap();
        let mut decoder = Decoder::new(&ebml_schema);
        let mut buffer = vec![];
        use std::io::Read;
        mkv.read_to_end(&mut buffer).unwrap();
        let elms = decoder.decode(buffer).unwrap();
        assert_eq!(elms.len(), 1024);
    }
}
