use chrono::{DateTime, Utc};
use derive_more::{Display, From, Into};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Element {
    // m
    MasterElement(MasterElement, ElementDetail),
    // u i f s 8 b d
    ChildElement(ChildElement, ElementDetail),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum MasterElement {
    MasterStartElement { ebml_id: EbmlId, unknown_size: bool },
    MasterEndElement { ebml_id: EbmlId },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ChildElement {
    // u
    UnsignedIntegerElement {
        ebml_id: EbmlId,
        value: u64,
    },
    // i
    IntegerElement {
        ebml_id: EbmlId,
        value: i64,
    },
    // f
    FloatElement {
        ebml_id: EbmlId,
        value: f64,
    },
    // s
    StringElement {
        ebml_id: EbmlId,
        value: Vec<u8>,
    },
    // 8
    Utf8Element {
        ebml_id: EbmlId,
        value: String,
    },
    // b
    BinaryElement {
        ebml_id: EbmlId,
        value: Vec<u8>,
    },
    // d
    DateElement {
        ebml_id: EbmlId,
        // signed 8 octets integer in nanoseconds with 0 indicating the precise
        // beginning of the millennium (at 2001-01-01T00:00:00,000000000 UTC)
        value: DateTime<Utc>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into, Display)]
pub struct EbmlId(pub i64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementDetail {
    // hex EBML ID
    pub ebml_id: EbmlId,
    // The level within an EBML tree that the element may occur at.
    // + is for a recursive level (can be its own child).
    // g: global element (can be found at any level)
    pub level: i64,
    // m u i f s 8 b d
    pub r#type: char,
    pub tag_start: usize,
    pub size_start: usize,
    pub content_start: usize,
    pub content_size: i64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct SimpleBlock {
    pub discardable: bool,
    pub frames: Vec<Vec<u8>>,
    pub invisible: bool,
    pub keyframe: bool,
    pub timecode: i64,
    pub track_number: i64,
}

pub trait SchemaDict {
    fn get(&self, ebml_id: EbmlId) -> Option<Schema>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Schema {
    pub r#type: char,
    pub level: i64,
}
