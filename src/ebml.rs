#![allow(clippy::unit_arg)]
use chrono::{DateTime, Utc};
use derivative::Derivative;
use derive_more::{Display, From, Into};
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

#[derive(Derivative, Arbitrary, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[derivative(Debug)]
pub enum Element {
    // m
    #[derivative(Debug = "transparent")]
    MasterElement(MasterElement),
    // u i f s 8 b d
    #[derivative(Debug = "transparent")]
    ChildElement(ChildElement),
}

// #[derive(Arbitrary)]
#[derive(Derivative, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[derivative(Debug)]
pub enum ElementDetail {
    // m
    #[derivative(Debug = "transparent")]
    MasterElement((MasterElement, ElementPosition)),
    // u i f s 8 b d
    #[derivative(Debug = "transparent")]
    ChildElement((ChildElement, ElementPosition)),
}

// #[derive(Arbitrary)]
// #[derive(Derivative, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
// #[derivative(Debug)]
// pub(crate) enum Tree {
//     #[derivative(Debug = "transparent")]
//     MasterElement((MasterStartElement, Vec<Tree>)),
//     #[derivative(Debug = "transparent")]
//     ChildElement((ChildElement, Vec<u8>)),
// }

#[derive(Derivative, Arbitrary, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[derivative(Debug)]
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

#[derive(Derivative, Arbitrary, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[derivative(Debug)]
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

#[derive(
    Derivative,
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
#[derivative(Debug = "transparent")]
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
pub struct SimpleBlock {
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
            ElementDetail::MasterElement((o, _)) => Element::MasterElement(o),
            ElementDetail::ChildElement((o, _)) => Element::ChildElement(o),
        }
    }
}
/*
impl From<(MasterStartElement, Vec<Tree>)> for Tree {
    fn from(o: (MasterStartElement, Vec<Tree>)) -> Tree {
        Tree::MasterElement(o)
    }
}

impl From<(ChildElement, Vec<u8>)> for Tree {
    fn from(o: (ChildElement, Vec<u8>)) -> Tree {
        Tree::ChildElement(o)
    }
}
*/
macro_rules! master_defs {
    ($ty:ident) => {
        impl From<$ty> for Element {
            fn from(o: $ty) -> Self {
                Element::$ty(o)
            }
        }

        impl From<($ty, ElementPosition)> for ElementDetail {
            fn from(o: ($ty, ElementPosition)) -> ElementDetail {
                ElementDetail::$ty(o)
            }
        }

        impl From<($ty, ElementPosition)> for Element {
            fn from(o: ($ty, ElementPosition)) -> Element {
                Element::$ty(o.0)
            }
        }
    };
}

master_defs!(MasterElement);
master_defs!(ChildElement);

macro_rules! master_defs2 {
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

        impl From<($ty, ElementPosition)> for ElementDetail {
            fn from(o: ($ty, ElementPosition)) -> ElementDetail {
                ElementDetail::MasterElement((o.0.into(), o.1))
            }
        }

        impl From<($ty, ElementPosition)> for Element {
            fn from(o: ($ty, ElementPosition)) -> Element {
                Element::MasterElement(o.0.into())
            }
        }
    };
}

master_defs2!(MasterStartElement);
master_defs2!(MasterEndElement);

macro_rules! child_defs {
    ($ty:ident, $ty2:ty) => {
        impl From<$ty> for Element {
            fn from(o: $ty) -> Element {
                Element::ChildElement(o.into())
            }
        }

        impl From<$ty> for ChildElement {
            fn from(o: $ty) -> ChildElement {
                ChildElement::$ty(o)
            }
        }

        impl From<(EbmlId, $ty2)> for $ty {
            fn from((ebml_id, value): (EbmlId, $ty2)) -> $ty {
                $ty { ebml_id, value }
            }
        }
    };
}

child_defs!(UnsignedIntegerElement, u64);
child_defs!(IntegerElement, i64);
child_defs!(FloatElement, f64);
child_defs!(StringElement, Vec<u8>);
child_defs!(Utf8Element, String);
child_defs!(BinaryElement, Vec<u8>);
child_defs!(DateElement, DateTime<Utc>);

fn arb_datetime() -> impl Strategy<Value = ::chrono::DateTime<::chrono::Utc>> {
    Just(::chrono::Utc::now())
}
