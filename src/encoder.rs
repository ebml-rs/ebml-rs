#![allow(dead_code)]
#![allow(unused_imports)]
use crate::ebml;
use crate::vint::{read_vint, UnrepresentableLengthError};
use err_derive::Error;

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
    pub fn encode<E: Into<ebml::Element>>(&mut self, elms: Vec<E>) -> Result<Vec<u8>, EncodeError> {
        for elm in elms {
            self.encode_chunk(elm.into())?;
        }
        let mut result = vec![];
        std::mem::swap(&mut self.queue, &mut result);
        Ok(result)
    }
    fn encode_chunk(&mut self, elm: ebml::Element) -> Result<(), EncodeError> {
        match elm {
            ebml::Element::MasterElement(ebml::MasterElement::MasterStartElement {
                ebml_id,
                unknown_size,
            }) => {
                self.start_tag(ebml_id, unknown_size)?;
            }
            ebml::Element::MasterElement(ebml::MasterElement::MasterEndElement { ebml_id }) => {
                self.end_tag(ebml_id)?;
            }
            ebml::Element::ChildElement(o) => {
                self.write_tag(o)?;
            }
        }
        Ok(())
    }
    fn write_tag(&mut self, _elm: ebml::ChildElement) -> Result<(), EncodeError> {
        Ok(())
    }
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
            ebml::MasterElement::MasterStartElement {
                ebml_id,
                unknown_size,
            },
            vec![],
        );
        if !self.stack.is_empty() {
            // self.stack.last_mut().unwrap().children.push(tree);
        } //else{
        self.stack.push(tree);
        //}
        Ok(())
    }
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
                ebml::MasterElement::MasterStartElement {
                    ebml_id: parent_ebml_id,
                    ..
                },
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
