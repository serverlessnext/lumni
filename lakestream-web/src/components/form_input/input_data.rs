use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use leptos::html::Input;
use leptos::*;

use super::form_field::FormField;

pub type InputElement = (
    NodeRef<Input>,
    RwSignal<Option<String>>,
    RwSignal<String>,
    Arc<InputData>,
);

pub type InputElements = HashMap<String, InputElement>;

#[derive(Clone)]
pub struct InputData {
    pub value: String,
    pub form_field: FormField,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl fmt::Debug for InputData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputData")
            .field("value", &self.value)
            .field("form_field", &self.form_field)
            .field("validator", &self.validator.is_some())
            .finish()
    }
}

impl InputData {
    pub fn new(
        value: String,
        form_field: FormField,
        validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    ) -> Self {
        Self {
            value,
            form_field,
            validator,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormInputField {
    pub name: String,
    pub input_data: InputData,
}

impl FormInputField {
    pub fn to_input_data(self) -> (String, InputData) {
        (self.name, self.input_data)
    }
}

pub fn create_input_elements(
    cx: Scope,
    updated_config: &HashMap<String, String>,
    default_config: &HashMap<String, InputData>,
) -> InputElements {
    updated_config
        .iter()
        .map(|(key, value)| {
            let error_signal = create_rw_signal(cx, None);
            let value_signal = create_rw_signal(cx, value.clone());
            let default_input_data = default_config
                .get(key)
                .expect("Default InputData to exist")
                .clone();
            (
                key.clone(),
                (
                    create_node_ref(cx),
                    error_signal,
                    value_signal,
                    Arc::new(default_input_data),
                ),
            )
        })
        .collect()
}
