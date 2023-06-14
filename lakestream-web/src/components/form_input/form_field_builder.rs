use std::sync::Arc;

use regex::Regex;

use super::helpers::validate_with_pattern;
use super::input_data::{FormInputField, InputData};
use crate::components::form_input::{FieldLabel, FieldType, FormField};

#[derive(Clone, Default)]
pub struct FormFieldBuilder {
    name: String,
    default: String,
    form_field: FormField,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl FormFieldBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            form_field: FormField {
                field_label: Some(FieldLabel::new("")),
                field_type: FieldType::Text { is_enabled: true },
            },
            ..Default::default()
        }
    }

    pub fn default(mut self, default: String) -> Self {
        self.default = default;
        self
    }

    pub fn text(mut self, is_enabled: bool) -> Self {
        self.form_field.field_type = FieldType::Text { is_enabled };
        self
    }

    pub fn secret(mut self, is_enabled: bool) -> Self {
        self.form_field.field_type = FieldType::Secret { is_enabled };
        self
    }

    pub fn password(mut self, is_enabled: bool) -> Self {
        self.form_field.field_type = FieldType::Password { is_enabled };
        self
    }

    pub fn label(mut self, text: Option<String>) -> Self {
        self.form_field.field_label = text.map(|l| FieldLabel::new(&l));
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
                self.form_field,
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

pub enum InputFieldPattern {
    PasswordChange,
    PasswordCheck,
}
