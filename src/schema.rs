use crate::ebml;
use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_SCHEMA_JSON: &str = include_str!("../schema.json");

pub trait SchemaDict<'a> {
    type Item: Schema;
    fn get(&'a self, ebml_id: ebml::EbmlId) -> Option<&'a Self::Item>;
}

pub trait Schema {
    fn name(&self) -> &str;
    fn r#type(&self) -> char;
    fn level(&self) -> i64;
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DefaultSchema(HashMap<String, DefaultSchemaEntry>);

impl Default for DefaultSchema {
    fn default() -> Self {
        let schema_str = DEFAULT_SCHEMA_JSON;
        serde_json::from_str::<Self>(schema_str).unwrap()
    }
}

impl<'a> SchemaDict<'a> for DefaultSchema {
    type Item = DefaultSchemaEntry;
    fn get(&'a self, ebml_id: ebml::EbmlId) -> Option<&'a Self::Item> {
        self.0.get(&format!("{}", ebml_id)).map(Into::into)
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DefaultSchemaEntry {
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

impl Schema for DefaultSchemaEntry {
    fn name(&self) -> &str {
        &self.name
    }
    fn r#type(&self) -> char {
        self.r#type.chars().next().unwrap()
    }
    fn level(&self) -> i64 {
        self.level
    }
}
