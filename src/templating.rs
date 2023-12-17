use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;

#[derive(Clone)]
pub struct TemplateString {
    string: String,
}

impl TemplateString {
    pub fn execute(&self, values: Vec<(&str, &str)>) -> String {
        let mut string = self.string.clone();
        values
            .iter()
            .for_each(|(k, v)| string = string.replace(&format!("{{{}}}", k), v));
        string
    }
}

impl Serialize for TemplateString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.string)
    }
}

struct TmplStrVisitor;

impl<'de> Visitor<'de> for TmplStrVisitor {
    type Value = TemplateString;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a valid template string")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(TemplateString { string: v })
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(TemplateString { string: v.into() })
    }
}

impl<'de> Deserialize<'de> for TemplateString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TmplStrVisitor)
    }
}

impl From<String> for TemplateString {
    fn from(value: String) -> Self {
        Self { string: value }
    }
}
