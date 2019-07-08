#![allow(unused_imports, dead_code)]
use crate::ebml;
use crate::vint::{write_vint, UnrepresentableValueError};
use err_derive::Error;
use log_derive::{logfn, logfn_inputs};

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error(display = "Io")]
    Io(#[error(cause)] std::io::Error),
    #[error(display = "UnknownEbmlId: {:?}", _0)]
    UnknownEbmlId(ebml::EbmlId),
    #[error(display = "EBML structure is broken")]
    Bloken,
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
            ebml::Element::MasterElement(ebml::MasterElement::MasterStartElement(
                ebml::MasterStartElement {
                    ebml_id,
                    unknown_size,
                },
            )) => {
                self.start_tag(ebml_id, unknown_size)?;
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
        // const buf = encode_tag(elm)?;
        // 親要素が閉じタグあり(isEnd)なら閉じタグが来るまで待つ(children queに入る)
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
    fn start_tag(&mut self, ebml_id: ebml::EbmlId, unknown_size: bool) -> Result<(), EncodeError> {
        let schema = self
            .schema
            .get(ebml_id)
            .ok_or_else(|| EncodeError::UnknownEbmlId(ebml_id))?;
        if unknown_size {
            // 不定長の場合はスタックに積まずに即時バッファに書き込む
            let data = unimplemented!(); //tools.encodeTag(tagId, new Buffer(0), elm.unknownSize);
            self.queue.append(data);
            return Ok(());
        }
        let tree = ebml::Tree::MasterElement(
            ebml::MasterStartElement {
                ebml_id,
                unknown_size,
            }
            .into(),
            vec![],
        );
        if !self.stack.is_empty() {
            // self.stack.last_mut().unwrap().children.push(tree);
        } //else{
        self.stack.push(tree);
        //}
        Ok(())
    }
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn end_tag(&mut self, ebml_id: ebml::EbmlId) -> Result<(), EncodeError> {
        let tree = self.stack.pop().ok_or_else(|| EncodeError::Bloken)?;
        match tree {
            ebml::Tree::ChildElenent(..) => {
                return Err(EncodeError::Bloken);
            }
            ebml::Tree::MasterElement(ebml::MasterElement::MasterEndElement { .. }, _) => {
                return Err(EncodeError::Bloken);
            }
            ebml::Tree::MasterElement(
                ebml::MasterElement::MasterStartElement(ebml::MasterStartElement {
                    ebml_id: parent_ebml_id,
                    ..
                }),
                _,
            ) => {
                if parent_ebml_id != ebml_id {
                    return Err(EncodeError::Bloken);
                }
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

/*
fn encode_master_tag(o: ebml::MasterElement) -> Result<Vec<u8>, EncodeTagError> {
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
}
fn encode_child_tag(elm: ebml::ChildElement) -> Result<Vec<u8>, EncodeTagError> {
    use std::convert::TryFrom;
    let (ebml_id, body) = match elm {
        ebml::ChildElement::BinaryElement{ebml_id, value} => {
            (ebml_id, vec![])
        },
        ebml::ChildElement::DateElement{ebml_id, value} => {
            (ebml_id, vec![])
        },
        ebml::ChildElement::FloatElement{ebml_id, value} => {
            (ebml_id, vec![])
        },
        ebml::ChildElement::IntegerElement{ebml_id, value} => {
            (ebml_id, vec![])
        },
        ebml::ChildElement::StringElement{ebml_id, value} => {
            (ebml_id, vec![])
        },
        ebml::ChildElement::UnsignedIntegerElement{ebml_id, value} => {
            (ebml_id, vec![])
        },
        ebml::ChildElement::Utf8Element{ebml_id, value} => {
            (ebml_id, vec![])
        },
    };
    Ok(vec![ebml_id, write_vint(i64::try_from(body.len())?), body])
}
*/

/*
export function encodeValueToBuffer(elm: EBML.MasterElement): EBML.MasterElement;
export function encodeValueToBuffer(elm: EBML.ChildElementsValue): EBML.ChildElementBuffer;
export function encodeValueToBuffer(elm: EBML.EBMLElementValue): EBML.EBMLElementBuffer {
  let data = new Buffer(0);
  if(elm.type === "m"){ return elm; }
  switch(elm.type){
    case "u": data = createUIntBuffer(elm.value); break;
    case "i": data = createIntBuffer(elm.value); break;
    case "f": data = createFloatBuffer(elm.value); break;
    case "s": data = new Buffer(elm.value, 'ascii'); break;
    case "8": data = new Buffer(elm.value, 'utf8'); break;
    case "b": data = elm.value; break;
    case "d": data = new Int64BE(elm.value.getTime().toString()).toBuffer(); break;
  }
  return Object.assign({}, elm, {data});
}

export function createUIntBuffer(value: number): Buffer {
  // Big-endian, any size from 1 to 8
  // but js number is float64, so max 6 bit octets
  let bytes: 1|2|3|4|5|6 = 1;
  for(; value >= Math.pow(2, 8*bytes); bytes++){}
  if(bytes >= 7){
    console.warn("7bit or more bigger uint not supported.");
    return new Uint64BE(value).toBuffer();
  }
  const data = new Buffer(bytes);
  data.writeUIntBE(value, 0, bytes);
  return data;
}

export function createIntBuffer(value: number): Buffer {
  // Big-endian, any size from 1 to 8 octets
  // but js number is float64, so max 6 bit
  let bytes: 1|2|3|4|5|6 = 1;
  for(; value >= Math.pow(2, 8*bytes); bytes++){}
  if(bytes >= 7){
    console.warn("7bit or more bigger uint not supported.");
    return new Int64BE(value).toBuffer();
  }
  const data = new Buffer(bytes);
  data.writeIntBE(value, 0, bytes);
  return data;
}

export function createFloatBuffer(value: number, bytes: 4|8 = 8): Buffer {
  // Big-endian, defined for 4 and 8 octets (32, 64 bits)
  // js number is float64 so 8 bytes.
  if(bytes === 8){
    // 64bit
    const data = new Buffer(8);
    data.writeDoubleBE(value, 0);
    return data;
  }else if(bytes === 4){
    // 32bit
    const data = new Buffer(4);
    data.writeFloatBE(value, 0);
    return data;
  }else{
    throw new Error("float type bits must 4bytes or 8bytes");
  }
}

export function convertEBMLDateToJSDate(int64str: number | string | Date): Date {
  if(int64str instanceof Date){ return int64str; }
  return new Date(new Date("2001-01-01T00:00:00.000Z").getTime() +  (Number(int64str)/1000/1000));
}

*/
