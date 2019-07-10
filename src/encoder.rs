#![allow(unused_imports, dead_code)]
use crate::ebml;
use crate::schema::{Schema, SchemaDict};
use crate::vint::{write_vint, UnrepresentableValueError, WriteVintExt};
use byteorder::{BigEndian, WriteBytesExt};
use err_derive::Error;
use log::debug;
use log_derive::{logfn, logfn_inputs};
use std::convert::TryFrom;

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error(display = "Io")]
    Io(#[error(cause)] std::io::Error),
    #[error(display = "UnknownEbmlId: {:?}", _0)]
    UnknownEbmlId(ebml::EbmlId),
    #[error(display = "EBML structure is broken")]
    Bloken,
    #[error(display = "EncodeTagError")]
    EncodeTag(#[error(cause)] EncodeTagError),
}

impl From<std::io::Error> for EncodeError {
    fn from(o: std::io::Error) -> EncodeError {
        EncodeError::Io(o)
    }
}

impl From<EncodeTagError> for EncodeError {
    fn from(o: EncodeTagError) -> EncodeError {
        EncodeError::EncodeTag(o)
    }
}

pub struct Encoder<'a, D: SchemaDict<'a>> {
    schema: &'a D,
    stack: Vec<(ebml::MasterStartElement, Vec<u8>)>,
    // c
    // c
    // m
    // + c
    // + c
    // + m
    // | + c
    // | + m
    // | | + c
    // | + c
    // + c
    // c
    queue: Vec<u8>,
}

impl<'a, D: SchemaDict<'a>> Encoder<'a, D> {
    pub fn new(schema: &'a D) -> Self {
        Self {
            schema,
            stack: vec![],
            queue: vec![],
        }
    }
    #[logfn(ok = "TRACE", err = "ERROR")]
    pub fn encode<E: Into<ebml::Element>>(&mut self, elms: Vec<E>) -> Result<Vec<u8>, EncodeError> {
        for elm in elms {
            self.encode_chunk(elm.into())?;
        }
        let mut result = vec![];
        std::mem::swap(&mut self.queue, &mut result);
        Ok(result)
    }
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn encode_chunk(&mut self, elm: ebml::Element) -> Result<(), EncodeError> {
        match elm {
            ebml::Element::MasterElement(ebml::MasterElement::MasterStartElement(o)) => {
                self.start_tag(o)?;
            }
            ebml::Element::MasterElement(ebml::MasterElement::MasterEndElement(
                ebml::MasterEndElement { ebml_id },
            )) => {
                self.end_tag(ebml_id)?;
            }
            ebml::Element::ChildElement(o) => {
                self.write_tag(o)?;
            }
        }
        Ok(())
    }
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn write_tag(&mut self, elm: ebml::ChildElement) -> Result<(), EncodeError> {
        let mut data = encode_child_tag(elm)?;
        // 親要素が閉じタグありなら閉じタグが来るまで待つ(master stack queueに入る)
        if !self.stack.is_empty() {
            let last = self.stack.last_mut().unwrap();
            last.1.append(&mut data);
            return Ok(());
        }
        self.queue.append(&mut data);
        Ok(())
    }
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn start_tag(&mut self, o: ebml::MasterStartElement) -> Result<(), EncodeError> {
        let schema = self
            .schema
            .get(o.ebml_id)
            .ok_or_else(|| EncodeError::UnknownEbmlId(o.ebml_id))?;
        let _level = schema.level();
        if o.unknown_size {
            // 不定長の場合はスタックに積まずに即時バッファに書き込む
            let mut data = encode_master_tag(o, vec![])?;
            self.queue.append(&mut data);
            return Ok(());
        }
        let tree = (o, vec![]);
        // スタックに積む
        self.stack.push(tree);
        Ok(())
    }
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn end_tag(&mut self, ebml_id: ebml::EbmlId) -> Result<(), EncodeError> {
        // このスタックの大きさが確定した
        let (o, buf) = self.stack.pop().ok_or_else(|| EncodeError::Bloken)?;
        // opening tag と closing tag の id が一致するか確認
        if o.ebml_id != ebml_id {
            return Err(EncodeError::Bloken);
        }
        let mut data = encode_master_tag(o, buf)?;
        if !self.stack.is_empty() {
            let last = self.stack.last_mut().unwrap();
            last.1.append(&mut data);
            return Ok(());
        }
        self.queue.append(&mut data);
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum EncodeTagError {
    #[error(display = "UnrepresentableValue")]
    UnrepresentableValue(#[error(cause)] UnrepresentableValueError),
    #[error(display = "TryFromIntError")]
    TryFromIntError(#[error(cause)] std::num::TryFromIntError),
}

impl From<std::num::TryFromIntError> for EncodeTagError {
    fn from(o: std::num::TryFromIntError) -> EncodeTagError {
        EncodeTagError::TryFromIntError(o)
    }
}

impl From<UnrepresentableValueError> for EncodeTagError {
    fn from(o: UnrepresentableValueError) -> EncodeTagError {
        EncodeTagError::UnrepresentableValue(o)
    }
}

#[logfn_inputs(TRACE)]
#[logfn(ok = "TRACE", err = "ERROR")]
fn encode_master_tag(
    o: ebml::MasterStartElement,
    mut body: Vec<u8>,
) -> Result<Vec<u8>, EncodeTagError> {
    let mut size_buffer = if o.unknown_size {
        // 0b_01ff_ffff_ffff_ffff
        vec![
            0b_0000_0001,
            0b_1111_1111,
            0b_1111_1111,
            0b_1111_1111,
            0b_1111_1111,
            0b_1111_1111,
            0b_1111_1111,
            0b_1111_1111,
        ]
    } else {
        write_vint(i64::try_from(body.len())?)?
    };
    let mut buf2 = vec![];
    buf2.write_int::<BigEndian>(o.ebml_id.0, 1).unwrap();
    buf2.append(&mut size_buffer);
    buf2.append(&mut body);
    Ok(buf2)
}

#[logfn_inputs(TRACE)]
#[logfn(ok = "TRACE", err = "ERROR")]
fn encode_child_tag(elm: ebml::ChildElement) -> Result<Vec<u8>, EncodeTagError> {
    Ok(match elm {
        ebml::ChildElement::BinaryElement(o) => o.into(),
        ebml::ChildElement::DateElement(o) => o.into(),
        ebml::ChildElement::FloatElement(o) => o.into(),
        ebml::ChildElement::IntegerElement(o) => o.into(),
        ebml::ChildElement::StringElement(o) => o.into(),
        ebml::ChildElement::UnsignedIntegerElement(o) => o.into(),
        ebml::ChildElement::Utf8Element(o) => o.into(),
    })
}

impl From<ebml::EbmlId> for Vec<u8> {
    #[allow(clippy::int_plus_one)] 
    fn from(ebml_id: ebml::EbmlId) -> Self {
        // bits, big-endian
        // 1xxx xxxx                                  - Class A IDs (2^7 -1 possible values) (base 0x8X)
        // 01xx xxxx  xxxx xxxx                       - Class B IDs (2^14-1 possible values) (base 0x4X 0xXX)
        // 001x xxxx  xxxx xxxx  xxxx xxxx            - Class C IDs (2^21-1 possible values) (base 0x2X 0xXX 0xXX)
        // 0001 xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx - Class D IDs (2^28-1 possible values) (base 0x1X 0xXX 0xXX 0xXX)
        let mut buf2 = vec![];
        if ebml_id.0 <= i64::pow(2, 7) - 1 {
            buf2.write_int::<BigEndian>(ebml_id.0, 1).unwrap();
            *buf2.get_mut(0).unwrap() |= 0b_1000_0000;
        } else if ebml_id.0 <= i64::pow(2, 14) - 1 {
            buf2.write_int::<BigEndian>(ebml_id.0, 2).unwrap();
            *buf2.get_mut(0).unwrap() |= 0b_0100_0000;
        } else if ebml_id.0 <= i64::pow(2, 21) - 1 {
            buf2.write_int::<BigEndian>(ebml_id.0, 3).unwrap();
            *buf2.get_mut(0).unwrap() |= 0b_0010_0000;
        } else if ebml_id.0 <= i64::pow(2, 28) - 1 {
            buf2.write_int::<BigEndian>(ebml_id.0, 4).unwrap();
            *buf2.get_mut(0).unwrap() |= 0b_0001_0000;
        }
        buf2
    }
}

impl From<ebml::UnsignedIntegerElement> for Vec<u8> {
    fn from(elm: ebml::UnsignedIntegerElement) -> Self {
        // Big-endian, any size from 1 to 8
        let mut bytes: usize = 1;
        while elm.value >= u64::pow(2, 8 * u32::try_from(bytes).unwrap()) {
            bytes += 1;
        }
        let mut buf = vec![0; bytes];
        buf.write_uint::<BigEndian>(elm.value, bytes).unwrap();
        let mut buf2 = vec![];
        buf2.append(&mut elm.ebml_id.into());
        buf2.append(&mut write_vint(i64::try_from(buf.len()).unwrap()).unwrap());
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::IntegerElement> for Vec<u8> {
    fn from(elm: ebml::IntegerElement) -> Self {
        // Big-endian, any size from 1 to 8 octets
        let mut bytes: usize = 1;
        while elm.value >= i64::pow(2, 8 * u32::try_from(bytes).unwrap()) {
            bytes += 1;
        }
        let mut buf = vec![0; bytes];
        buf.write_int::<BigEndian>(elm.value, bytes).unwrap();
        let mut buf2 = vec![];
        buf2.append(&mut elm.ebml_id.into());
        buf2.append(&mut write_vint(i64::try_from(buf.len()).unwrap()).unwrap());
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::FloatElement> for Vec<u8> {
    fn from(elm: ebml::FloatElement) -> Self {
        // Big-endian, defined for 4 and 8 octets (32, 64 bits)
        // currently 64bit support only
        let mut buf = vec![0; 8];
        buf.write_f64::<BigEndian>(elm.value).unwrap();
        let mut buf2 = vec![];
        buf2.append(&mut elm.ebml_id.into());
        buf2.append(&mut write_vint(i64::try_from(buf.len()).unwrap()).unwrap());
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::StringElement> for Vec<u8> {
    fn from(elm: ebml::StringElement) -> Self {
        let mut buf = elm.value.clone();
        let mut buf2 = vec![];
        buf2.append(&mut elm.ebml_id.into());
        buf2.append(&mut write_vint(i64::try_from(buf.len()).unwrap()).unwrap());
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::Utf8Element> for Vec<u8> {
    fn from(elm: ebml::Utf8Element) -> Self {
        let mut buf = elm.value.as_bytes().to_vec();
        let mut buf2 = vec![];
        buf2.append(&mut elm.ebml_id.into());
        buf2.append(&mut write_vint(i64::try_from(buf.len()).unwrap()).unwrap());
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::BinaryElement> for Vec<u8> {
    fn from(elm: ebml::BinaryElement) -> Self {
        let mut buf = elm.value.clone();
        let mut buf2 = vec![];
        buf2.append(&mut elm.ebml_id.into());
        buf2.append(&mut write_vint(i64::try_from(buf.len()).unwrap()).unwrap());
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::DateElement> for Vec<u8> {
    fn from(elm: ebml::DateElement) -> Self {
        // nano second; Date.UTC(2001,1,1,0,0,0,0) === 980985600000
        // new Date("2001-01-01T00:00:00.000Z").getTime() = 978307200000
        // Date - signed 8 octets integer in nanoseconds with 0 indicating
        // the precise beginning of the millennium (at 2001-01-01T00:00:00,000000000 UTC)
        let nanos = elm.value.timestamp_nanos() + 978_307_200 * 1000 * 1000 * 1000;
        let mut buf = vec![0; 8];
        buf.write_int::<BigEndian>(nanos, 8).unwrap();
        let mut buf2 = vec![];
        buf2.append(&mut elm.ebml_id.into());
        buf2.append(&mut write_vint(i64::try_from(buf.len()).unwrap()).unwrap());
        buf2.append(&mut buf);
        buf2
    }
}

#[test]
fn test_tag_encoder() {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "trace");
    env_logger::try_init().ok();

    let schema = crate::schema::DefaultSchema::default();
    let mut decoder = crate::decoder::Decoder::new(&schema);
    // Name
    let buf: Vec<u8> = ebml::Utf8Element {
        ebml_id: 21358.into(),
        value: "a".to_string(),
    }
    .into();
    println!("{:?}", buf);
    println!("{:?}", decoder.decode(buf));
}
