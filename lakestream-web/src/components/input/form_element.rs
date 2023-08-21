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

#[derive(Clone, Default, Debug)]
pub struct FieldPlaceholder {
    text: String,
}

impl FieldPlaceholder {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }
}


#[derive(Clone)]
pub struct FormElement {
    pub name: String,
    pub field_content_type: FieldContentType,
    pub field_label: Option<FieldLabel>,
    pub field_placeholder: Option<FieldPlaceholder>,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    pub buffer_data: String, // data always gets loaded in here first
    pub is_enabled: bool,
}

impl fmt::Debug for FormElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormElement")
            .field("name", &self.name)
            .field("field_content_type", &self.field_content_type)
            .field("field_label", &self.field_label)
            .field("field_placeholder", &self.field_placeholder)
            .field("validator", &self.validator.is_some())
            .field("buffer_data", &self.buffer_data)
            .field("is_enabled", &self.is_enabled)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct FormElementState {
    pub display_error: RwSignal<Option<String>>,
    pub display_value: RwSignal<DisplayValue>,
    pub schema: Arc<FormElement>,
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
    pub fn as_text(&self) -> String {
        match self {
            DisplayValue::Text(text) => text.clone(),
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            DisplayValue::Text(text) => text.is_empty(),
        }
    }
}
