#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Element {
    // m
    MasterElement(MasterElement, Detail),
    // u i f s 8 b d
    ChildElement(ChildElement, Detail),
}
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum MasterElement {
    MasterStartElement { ebml_id: i64, unknown_size: bool },
    MasterEndElement { ebml_id: i64 },
}
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ChildElement {
    // u
    UnsignedIntegerElement {
        ebml_id: i64,
        value: u64,
    },
    // i
    IntegerElement {
        ebml_id: i64,
        value: i64,
    },
    // f
    FloatElement {
        ebml_id: i64,
        value: f64,
    },
    // s
    StringElement {
        ebml_id: i64,
        value: Vec<u8>,
    },
    // 8
    Utf8Element {
        ebml_id: i64,
        value: String,
    },
    // b
    BinaryElement {
        ebml_id: i64,
        value: Vec<u8>,
    },
    // d
    DateElement {
        ebml_id: i64,
        // signed 8 octets integer in nanoseconds with 0 indicating the precise
        // beginning of the millennium (at 2001-01-01T00:00:00,000000000 UTC)
        value: i64,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Detail {
    // hex EBML ID
    pub ebml_id: i64,
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
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Schema {
    pub name: String,
    pub cppname: Option<String>,
    pub level: i64,
    pub multiple: Option<String>,
    pub r#type: char,
    pub description: String,
}
