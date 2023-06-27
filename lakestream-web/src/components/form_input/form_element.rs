use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use leptos::*;

use super::field_type::{DocumentType, FieldType};

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

#[allow(dead_code)] // silence non-use warning for now
#[derive(Debug, Clone)]
pub enum FormElement {
    TextBox(ElementData),
    TextArea(ElementData),
    NestedForm(ElementData),
}

#[derive(Clone, Debug)]
pub struct ElementData {
    pub name: String,
    pub element_type: ElementDataType,
    pub is_enabled: bool,
}

#[allow(dead_code)] // silence non-use warning for now
#[derive(Debug, Clone)]
pub enum ElementDataType {
    TextData(TextData),
    BinaryData(BinaryData),
    DocumentData(DocumentData),
}

#[derive(Clone)]
pub struct TextData {
    pub field_type: FieldType,
    pub field_label: Option<FieldLabel>,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    pub buffer_data: String, // data always gets loaded in here first
}

impl fmt::Debug for TextData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextData")
            .field("field_type", &self.field_type)
            .field("field_label", &self.field_label)
            .field("validator", &self.validator.is_some())
            .field("buffer_data", &self.buffer_data)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct BinaryData {
    pub field_label: Option<FieldLabel>,
    pub buffer_data: Vec<u8>,
}

#[derive(Clone)]
pub struct DocumentData {
    pub document_type: DocumentType,
    pub field_label: Option<FieldLabel>,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    pub buffer_data: String,
}

impl fmt::Debug for DocumentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocumentData")
            .field("document_type", &self.document_type)
            .field("field_label", &self.field_label)
            .field("validator", &self.validator.is_some())
            .field("buffer_data", &self.buffer_data)
            .finish()
    }
}

pub type FormState = HashMap<String, FormElementState>;

#[derive(Clone, Debug)]
pub struct FormElementState {
    pub display_error: RwSignal<Option<String>>,
    pub display_value: RwSignal<DisplayValue>,
    pub schema: Arc<ElementData>,
}

impl FormElementState {
    pub fn read_display_value(&self) -> DisplayValue {
        self.display_value.get_untracked()
    }
}

#[derive(Clone, Debug)]
pub enum DisplayValue {
    Text(String),
    Binary(Vec<u8>),
}

impl DisplayValue {
    pub fn is_empty(&self) -> bool {
        match self {
            DisplayValue::Text(text) => text.is_empty(),
            DisplayValue::Binary(data) => data.is_empty(),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            DisplayValue::Text(text) => Some(text),
            DisplayValue::Binary(_) => None,
        }
    }
}
