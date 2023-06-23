use std::fmt;
use std::sync::Arc;
use std::collections::HashMap;

use leptos::*;
use super::FieldType;

#[derive(Clone, Default, Debug)]
pub struct FieldLabel {
    text: String,
}

impl FieldLabel {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }
}

#[derive(Debug, Clone)]
pub enum FormElement {
    TextBox(ElementData),
    TextArea(ElementData),
}


#[derive(Clone, Debug)]
pub struct ElementData {
    pub name: String,
    pub element_type: ElementDataType,
    pub is_enabled: bool,
}



#[derive(Debug, Clone)]
pub enum ElementDataType {
    TextData(TextData),
    BinaryData(BinaryData),
    DocumentData(DocumentData),
    // and so on for other types of data
}

#[derive(Clone)]
pub struct TextData {
    pub value: String,
    pub field_type: FieldType,
    pub field_label: Option<FieldLabel>,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl fmt::Debug for TextData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextData")
            .field("value", &self.value)
            .field("field_type", &self.field_type)
            .field("field_label", &self.field_label)
            .field("validator", &self.validator.is_some())
            .finish()
    }
}


#[derive(Debug, Clone)]
pub struct BinaryData {
    pub value: Vec<u8>,  // binary data usually represented as byte array
    // other fields specific to binary data
}

#[derive(Debug, Clone)]
pub struct DocumentData {
    pub value: serde_json::Value,  // using the serde_json library for JSON values
    // other fields specific to JSON data
}

#[derive(Clone, Debug)]
pub struct FormElementState {
    pub error: RwSignal<Option<String>>,
    pub value: RwSignal<String>,
    pub schema: Arc<ElementData>,
}

pub type FormState = HashMap<String, FormElementState>;

