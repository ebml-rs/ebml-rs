#![allow(clippy::unit_arg)]
use chrono::{DateTime, Utc};
use derivative::Derivative;
use derive_more::{Display, From, Into};
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

#[derive(Derivative)]
#[derivative(Debug)]
#[derive(Arbitrary, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Element {
    // m
    #[derivative(Debug = "transparent")]
    MasterElement(MasterElement),
    // u i f s 8 b d
    #[derivative(Debug = "transparent")]
    ChildElement(ChildElement),
}

// #[derive(Arbitrary)]
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ElementDetail {
    // m
    MasterElement(MasterElement, ElementPosition),
    // u i f s 8 b d
    ChildElement(ChildElement, ElementPosition),
}

// #[derive(Arbitrary)]
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub(crate) enum Tree {
    MasterElement(MasterElement, Vec<Tree>),
    ChildElenent(ChildElement),
}

#[derive(Derivative)]
#[derivative(Debug)]
#[derive(Arbitrary, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum MasterElement {
    #[derivative(Debug = "transparent")]
    MasterStartElement(MasterStartElement),
    #[derivative(Debug = "transparent")]
    MasterEndElement(MasterEndElement),
}

#[derive(
    Arbitrary, Debug, Clone, PartialEq, PartialOrd, Copy, Eq, Ord, Hash, Serialize, Deserialize,
)]
pub struct MasterStartElement {
    pub ebml_id: EbmlId,
    pub unknown_size: bool,
}

#[derive(
    Arbitrary, Debug, Clone, PartialEq, PartialOrd, Copy, Eq, Ord, Hash, Serialize, Deserialize,
)]
pub struct MasterEndElement {
    pub ebml_id: EbmlId,
}

#[derive(Derivative)]
#[derivative(Debug)]
#[derive(Arbitrary, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ChildElement {
    // u
    #[derivative(Debug = "transparent")]
    UnsignedIntegerElement(UnsignedIntegerElement),
    // i
    #[derivative(Debug = "transparent")]
    IntegerElement(IntegerElement),
    // f
    #[derivative(Debug = "transparent")]
    FloatElement(FloatElement),
    // s
    #[derivative(Debug = "transparent")]
    StringElement(StringElement),
    // 8
    #[derivative(Debug = "transparent")]
    Utf8Element(Utf8Element),
    // b
    #[derivative(Debug = "transparent")]
    BinaryElement(BinaryElement),
    // d
    #[derivative(Debug = "transparent")]
    DateElement(DateElement),
}

#[derive(
    Arbitrary, Debug, Clone, PartialEq, PartialOrd, Copy, Eq, Ord, Hash, Serialize, Deserialize,
)]
pub struct UnsignedIntegerElement {
    pub ebml_id: EbmlId,
    pub value: u64,
}

#[derive(
    Arbitrary, Debug, Clone, PartialEq, PartialOrd, Copy, Eq, Ord, Hash, Serialize, Deserialize,
)]
pub struct IntegerElement {
    pub ebml_id: EbmlId,
    pub value: i64,
}

#[derive(Arbitrary, Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FloatElement {
    pub ebml_id: EbmlId,
    pub value: f64,
}

#[derive(Arbitrary, Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct StringElement {
    pub ebml_id: EbmlId,
    pub value: Vec<u8>,
}

#[derive(Arbitrary, Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Utf8Element {
    pub ebml_id: EbmlId,
    pub value: String,
}

#[derive(Arbitrary, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BinaryElement {
    pub ebml_id: EbmlId,
    pub value: Vec<u8>,
}

impl std::fmt::Debug for BinaryElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BinaryElement {{ ebml_id: {:?}, value: Vec<u8; {:?}> }}",
            self.ebml_id,
            self.value.len()
        )
    }
}

#[derive(
    Arbitrary, Debug, Clone, PartialEq, PartialOrd, Copy, Eq, Ord, Hash, Serialize, Deserialize,
)]
pub struct DateElement {
    pub ebml_id: EbmlId,
    // signed 8 octets integer in nanoseconds with 0 indicating the precise
    // beginning of the millennium (at 2001-01-01T00:00:00,000000000 UTC)
    #[proptest(strategy = "arb_datetime()")]
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub value: DateTime<Utc>,
}

#[derive(Derivative)]
#[derivative(Debug = "transparent")]
#[derive(
    Arbitrary,
    Clone,
    PartialEq,
    PartialOrd,
    Copy,
    Eq,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    From,
    Into,
    Display,
)]
pub struct EbmlId(pub i64);

#[derive(
    Arbitrary, Debug, Clone, PartialEq, PartialOrd, Copy, Eq, Ord, Hash, Serialize, Deserialize,
)]
pub struct ElementPosition {
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

#[derive(Arbitrary, Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub(crate) struct SimpleBlock {
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

#[derive(
    Arbitrary, Debug, Clone, PartialEq, PartialOrd, Copy, Eq, Ord, Hash, Serialize, Deserialize,
)]
pub struct Schema {
    pub r#type: char,
    pub level: i64,
}

impl From<ElementDetail> for Element {
    fn from(o: ElementDetail) -> Self {
        match o {
            ElementDetail::MasterElement(o, _) => Element::MasterElement(o),
            ElementDetail::ChildElement(o, _) => Element::ChildElement(o),
        }
    }
}

impl From<MasterElement> for Element {
    fn from(o: MasterElement) -> Self {
        Element::MasterElement(o)
    }
}

impl From<ChildElement> for Element {
    fn from(o: ChildElement) -> Self {
        Element::ChildElement(o)
    }
}

macro_rules! master_defs {
    ($ty:ident) => {
        impl From<$ty> for Element {
            fn from(o: $ty) -> Self {
                Element::MasterElement(o.into())
            }
        }

        impl From<$ty> for MasterElement {
            fn from(o: $ty) -> Self {
                MasterElement::$ty(o)
            }
        }
    };
}

master_defs!(MasterStartElement);
master_defs!(MasterEndElement);

macro_rules! child_defs {
    ($ty:ident) => {
        impl From<$ty> for Element {
            fn from(o: $ty) -> Self {
                Element::ChildElement(o.into())
            }
        }

        impl From<$ty> for ChildElement {
            fn from(o: $ty) -> Self {
                ChildElement::$ty(o)
            }
        }
    };
}

child_defs!(UnsignedIntegerElement);
child_defs!(IntegerElement);
child_defs!(FloatElement);
child_defs!(StringElement);
child_defs!(Utf8Element);
child_defs!(BinaryElement);
child_defs!(DateElement);

fn arb_datetime() -> impl Strategy<Value = ::chrono::DateTime<::chrono::Utc>> {
    Just(::chrono::Utc::now())
}
