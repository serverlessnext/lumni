use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use leptos::*;

use super::FieldContentType;

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
}

#[derive(Clone, Debug)]
pub struct ElementData {
    pub name: String,
    pub element_type: ElementDataType,
    pub is_enabled: bool,
}

#[derive(Debug, Clone)]
pub enum ElementDataType {
    TextData(FormElementData),
}

#[derive(Clone)]
pub struct FormElementData {
    pub field_content_type: FieldContentType,
    pub field_label: Option<FieldLabel>,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    pub buffer_data: String, // data always gets loaded in here first
}

impl fmt::Debug for FormElementData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormElementData")
            .field("field_content_type", &self.field_content_type)
            .field("field_label", &self.field_label)
            .field("validator", &self.validator.is_some())
            .field("buffer_data", &self.buffer_data)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct FormState {
    elements: HashMap<String, FormElementState>,
}

impl FormState {
    pub fn new(elements: HashMap<String, FormElementState>) -> Self {
        Self {
            elements,
        }
    }

    pub fn elements(&self) -> &HashMap<String, FormElementState> {
        &self.elements
    }
}

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
}

impl DisplayValue {
    pub fn is_empty(&self) -> bool {
        match self {
            DisplayValue::Text(text) => text.is_empty(),
        }
    }
}
