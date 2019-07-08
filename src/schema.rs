use crate::decoder::Decoder;
use crate::ebml;
use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_SCHEMA_JSON: &str = include_str!("../schema.json");

impl Default for Decoder<Dict> {
    fn default() -> Self {
        let schema_str = DEFAULT_SCHEMA_JSON;
        let o = serde_json::from_str(schema_str).unwrap();
        Self::new(o)
    }
}

impl ebml::SchemaDict for Dict {
    fn get(&self, ebml_id: ebml::EbmlId) -> Option<ebml::Schema> {
        self.0
            .get(&format!("{}", ebml_id).to_string())
            .map(Into::into)
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Dict(HashMap<String, Entry>);

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub r#type: String,
    pub level: i64,
    pub description: String,
    pub cppname: Option<String>,
    pub multiple: Option<bool>,
    pub webm: Option<bool>,
    pub minver: Option<i64>,
    pub bytesize: Option<i64>,
    pub range: Option<String>,
    pub default: Option<serde_json::Value>,
}

impl From<&Entry> for ebml::Schema {
    fn from(o: &Entry) -> ebml::Schema {
        ebml::Schema {
            level: o.level,
            r#type: o.r#type.chars().next().unwrap(),
        }
    }
}
