use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use leptos::html::Input;
use leptos::*;

use super::{FieldLabel, FieldType};

pub type InputElement = (
    NodeRef<Input>,
    RwSignal<Option<String>>,
    RwSignal<String>,
    Arc<InputFieldData>,
);

pub type InputElements = HashMap<String, InputElement>;

#[derive(Debug, Clone)]
pub struct TextBox {
    pub name: String,
    pub input_data: InputFieldData,
}

#[derive(Clone)]
pub struct InputFieldData {
    pub name: String,
    pub value: String,
    pub field_type: FieldType,
    pub field_label: Option<FieldLabel>,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    pub is_enabled: bool,
}

impl fmt::Debug for InputFieldData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputFieldData")
            .field("name", &self.name)
            .field("value", &self.value)
            .field("field_type", &self.field_type)
            .field("field_label", &self.field_label)
            .field("validator", &self.validator.is_some())
            .field("is_enabled", &self.is_enabled)
            .finish()
    }
}
