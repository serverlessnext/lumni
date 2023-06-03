use std::sync::Arc;

use regex::Regex;

use super::form_input::{FormInputField, InputData, InputField};
use super::helpers::validate_with_pattern;

#[derive(Clone, Default)]
pub struct FormInputFieldBuilder {
    name: String,
    default: String,
    input_field: InputField,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

pub enum InputFieldPattern {
    PasswordChange,
    PasswordCheck,
}

impl FormInputFieldBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            input_field: InputField::new_text(true),
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

    pub fn with_pattern(pattern: InputFieldPattern) -> InputData {
        let builder = match pattern {
            InputFieldPattern::PasswordChange => {
                let password_pattern = Regex::new(r"^.{8,}$").unwrap();
                Self::new("PASSWORD")
                    .default("".to_string())
                    .password(true)
                    .validator(Some(Arc::new(validate_with_pattern(
                        password_pattern,
                        "Invalid password. Must be at least 8 characters."
                            .to_string(),
                    ))))
            }
            InputFieldPattern::PasswordCheck => {
                Self::new("PASSWORD").default("".to_string()).password(true)
            }
        };
        builder.build().to_input_data().1
    }
}
