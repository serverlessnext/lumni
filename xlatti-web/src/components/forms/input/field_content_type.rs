use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum FieldContentType {
    PlainText,
    Secret,   // can be hidden and un-hidden
    Password, // can not be unhidden
    TextArea, // TextArea is PlainText but with larger buffer/ field
}

impl FieldContentType {
    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Secret)
    }

    pub fn is_password(&self) -> bool {
        matches!(self, Self::Password)
    }

    pub fn is_plain_text(&self) -> bool {
        matches!(self, Self::PlainText)
    }

    pub fn is_text_area(&self) -> bool {
        matches!(self, Self::TextArea)
    }
}

impl Default for FieldContentType {
    fn default() -> Self {
        Self::PlainText
    }
}

impl FromStr for FieldContentType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PlainText" => Ok(FieldContentType::PlainText),
            "Secret" => Ok(FieldContentType::Secret),
            "Password" => Ok(FieldContentType::Password),
            "TextArea" => Ok(FieldContentType::TextArea),
            _ => Err(()),
        }
    }
}
