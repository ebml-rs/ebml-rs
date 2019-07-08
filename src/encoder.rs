#![allow(unused_imports, dead_code)]
use crate::ebml;
use crate::vint::{write_vint, UnrepresentableValueError, WriteVintExt};
use byteorder::{BigEndian, WriteBytesExt};
use err_derive::Error;
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

pub struct Encoder<D> {
    schema: D,
    stack: Vec<ebml::Tree>,
    queue: Vec<u8>,
}

impl<D: ebml::SchemaDict> Encoder<D> {
    pub fn new(schema: D) -> Self {
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
        let buf = encode_child_tag(elm)?;
        // 親要素が閉じタグありなら閉じタグが来るまで待つ(children queに入る)
        if !self.stack.is_empty() {
            let last = self.stack.last_mut().unwrap();
            // last.children.push({
            //     tagId,
            //     elm,
            //     children: <DataTree[]>[],
            //     data
            // });
            return Ok(());
        }
        // self.queue.append(data);
        Ok(())
    }
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn start_tag(&mut self, o: ebml::MasterStartElement) -> Result<(), EncodeError> {
        let schema = self
            .schema
            .get(o.ebml_id)
            .ok_or_else(|| EncodeError::UnknownEbmlId(o.ebml_id))?;
        if o.unknown_size {
            // 不定長の場合はスタックに積まずに即時バッファに書き込む
            let mut data = encode_master_tag(o, vec![])?;
            self.queue.append(&mut data);
            return Ok(());
        }
        let tree = (o, vec![]).into();
        if !self.stack.is_empty() {
            match self.stack.last_mut().unwrap() {
                ebml::Tree::MasterElement((_, ref mut children)) => {
                    children.push(tree);
                }
                _ => {
                    return Err(EncodeError::Bloken);
                }
            }
        } else {
            self.stack.push(tree);
        }
        Ok(())
    }
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn end_tag(&mut self, ebml_id: ebml::EbmlId) -> Result<(), EncodeError> {
        match self.stack.pop().ok_or_else(|| EncodeError::Bloken)? {
            ebml::Tree::ChildElement(..) => {
                return Err(EncodeError::Bloken);
            }
            ebml::Tree::MasterElement((
                ebml::MasterStartElement {
                    ebml_id: parent_ebml_id,
                    ..
                },
                children,
            )) => {
                if parent_ebml_id != ebml_id {
                    return Err(EncodeError::Bloken);
                }
                // children
            }
        }
        // const childTagDataBuffers = tree.children.reduce<Buffer[]>((lst, child)=>{
        //     if(child.data === null){ throw new Error("EBML structure is broken"); }
        //     return lst.concat(child.data);
        // }, []);
        // const childTagDataBuffer = tools.concat(childTagDataBuffers);
        // if(tree.elm.type === "m"){
        // tree.data = tools.encodeTag(tag.tagId, childTagDataBuffer, tag.elm.unknownSize);
        // }else{
        // tree.data = tools.encodeTag(tag.tagId, childTagDataBuffer);
        // }
        if self.stack.len() < 1 {
            // self.queue.append(tree.data);
        }
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

#[logfn_inputs(TRACE)]
#[logfn(ok = "TRACE", err = "ERROR")]
fn encode_master_tag(
    o: ebml::MasterStartElement,
    children: Vec<ebml::Tree>,
) -> Result<Vec<u8>, EncodeTagError> {
    Ok(vec![])
    /*
    use std::convert::TryFrom;
    let (ebml_id, body) = match o {
        ebml::MasterElement::MasterStartElement{ ebml_id, unknown_size } => {
            // unknownSize ? new Buffer('01ff_ffff_ffff_ffff', 'hex') :  writeVint(tagData.length),
            (ebml_id, vec![])
        },
        ebml::MasterElement::MasterEndElement{ ebml_id } => {
            (ebml_id, vec![])
        },
    };
    Ok(vec![ebml_id, write_vint(i64::try_from(body.len())?), body])
    */
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
        buf2.write_int::<BigEndian>(elm.ebml_id.0, 1).unwrap();
        buf2.write_vint(i64::try_from(buf.len()).unwrap()).unwrap();
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
        buf2.write_int::<BigEndian>(elm.ebml_id.0, 1).unwrap();
        buf2.write_vint(i64::try_from(buf.len()).unwrap()).unwrap();
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
        buf2.write_int::<BigEndian>(elm.ebml_id.0, 1).unwrap();
        buf2.write_vint(i64::try_from(buf.len()).unwrap()).unwrap();
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::StringElement> for Vec<u8> {
    fn from(elm: ebml::StringElement) -> Self {
        let mut buf = elm.value.clone();
        let mut buf2 = vec![];
        buf2.write_int::<BigEndian>(elm.ebml_id.0, 1).unwrap();
        buf2.write_vint(i64::try_from(buf.len()).unwrap()).unwrap();
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::Utf8Element> for Vec<u8> {
    fn from(elm: ebml::Utf8Element) -> Self {
        let mut buf = elm.value.as_bytes().to_vec();
        let mut buf2 = vec![];
        buf2.write_int::<BigEndian>(elm.ebml_id.0, 1).unwrap();
        buf2.write_vint(i64::try_from(buf.len()).unwrap()).unwrap();
        buf2.append(&mut buf);
        buf2
    }
}

impl From<ebml::BinaryElement> for Vec<u8> {
    fn from(elm: ebml::BinaryElement) -> Self {
        let mut buf = elm.value.clone();
        let mut buf2 = vec![];
        buf2.write_int::<BigEndian>(elm.ebml_id.0, 1).unwrap();
        buf2.write_vint(i64::try_from(buf.len()).unwrap()).unwrap();
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
        let mut bytes: usize = 1;
        while nanos >= i64::pow(2, 8 * u32::try_from(bytes).unwrap()) {
            bytes += 1;
        }
        let mut buf = vec![0; bytes];
        buf.write_int::<BigEndian>(nanos, bytes).unwrap();
        let mut buf2 = vec![];
        buf2.write_int::<BigEndian>(elm.ebml_id.0, 1).unwrap();
        buf2.write_vint(i64::try_from(buf.len()).unwrap()).unwrap();
        buf2.append(&mut buf);
        buf2
    }
}
