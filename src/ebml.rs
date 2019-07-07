
pub enum EBMLElementBuffer {
    MasterElement(MasterElement),
    ChildElement(ChildElement, Vec<u8>),
}
pub enum EBMLElementDetail {
    MasterElement(MasterElement, ElementDetail),
    ChildElement(ChildElement, Vec<u8>, ElementDetail),
}

pub enum EbmlElement {
    // m
    MasterElement(MasterElement),
    ChildElement(ChildElement),
}
pub enum ChildElement {
    // u
    UnsignedIntegerElement(UnsignedIntegerElement),
    // i
    IntegerElement(IntegerElement),
    // f
    FloatElement(FloatElement),
    // s
    StringElement(StringElement),
    // 8
    Utf8StringElement(Utf8StringElement),
    // b
    BinaryElement(BinaryElement),
    // d
    DateElement(DateElement),
}
pub struct MasterElement {
    pub name: String,
    pub isEnd: boolean,
    pub unknownSize: boolean,
}
struct UnsignedIntegerElement {
    pub name: String,
    pub value: u64,
}
pub struct IntegerElement {
    pub name: String,
    pub value: i64,
}
pub struct FloatElement {
    pub name: String,
    pub value: i64,
}
pub struct StringElement{
    pub name: String,
    pub value: Vec<u8>,
}
pub struct Utf8StringElement{
    pub name: String,
    pub value: String,
}
pub struct BufferElement{
    pub name: String,
    pub value: Vec<u8>,
}
pub struct DateElement{
    pub name: String,
    // signed 8 octets integer in nanoseconds with 0 indicating the precise
    // beginning of the millennium (at 2001-01-01T00:00:00,000000000 UTC)
    pub value: i64,
}
pub struct ElementDetail {
    pub schema: Schema,
  /**
   * hex EBML ID
   */
    pub EBML_ID: u64,
  /**
   * The level within an EBML tree that the element may occur at. 
   * + is for a recursive level (can be its own child). 
   * g: global element (can be found at any level)
   */
    pub level: i64,
  /**
   * このタグのバッファ全体における開始オフセット位置
   */
    pub tagStart: i64,
  /**
   * このタグのバッファ全体における終了オフセット位置
   */
    pub tagEnd: i64,
  /**
   * size vint start
   */
    pub sizeStart: i64,
  /**
   * size vint end
   */
    pub sizeEnd: i64
  /**
   * 要素の中身の開始位置
   */
    pub dataStart: i64,
  /**
   * 要素の中身の終了位置
   */
    pub dataEnd: i64,
  /**
   * dataEnd - dataStart
   */
    pub dataSize: i64,
}

pub struct SimpleBlock {
    pub discardable: bool,
    pub frames: Vec<Vec<u8>>,
    pub invisible: bool,
    pub keyframe: bool,
    pub timecode: i64,
    pub trackNumber: i64,
}

pub struct Schema {
    pub name: String,
    pub level: i64,
    pub type: String,
    pub description: String,
}
