use std::sync::Arc;

use super::{FormInputField, InputData, InputField};

#[derive(Clone, Default)]
pub struct FormInputFieldBuilder {
    name: String,
    default: String,
    input_field: InputField,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl FormInputFieldBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            input_field: InputField::new_text(true), // Default to Text field
            ..Default::default()
        }
    }

    pub fn default(mut self, default: String) -> Self {
        self.default = default;
        self
    }

    pub fn text(mut self, is_enabled: bool) -> Self {
        self.input_field = InputField::new_text(is_enabled);
        self
    }

    pub fn secret(mut self, is_enabled: bool) -> Self {
        self.input_field = InputField::new_secret(is_enabled);
        self
    }

    pub fn password(mut self, is_enabled: bool) -> Self {
        self.input_field = InputField::new_password(is_enabled);
        self
    }

    pub fn validator(
        mut self,
        validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    ) -> Self {
        self.validate_fn = validate_fn;
        self
    }

    pub fn build(self) -> FormInputField {
        FormInputField {
            name: self.name,
            input_data: InputData::new(
                self.default,
                self.input_field,
                self.validate_fn,
            ),
        }
    }
}

