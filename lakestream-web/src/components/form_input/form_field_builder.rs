use std::sync::Arc;

use regex::Regex;
use super::InputFieldData;

use super::helpers::validate_with_pattern;
use crate::components::form_input::{FieldLabel, FieldType, FormElement};


#[derive(Clone)]
pub struct FormFieldBuilder {
    name: String,
    default: String,
    field_type: FieldType,
    field_label: Option<FieldLabel>,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    is_enabled: bool,
}

impl Default for FormFieldBuilder {
    fn default() -> Self {
        Self {
            name: String::new(),
            default: String::new(),
            field_type: FieldType::Text,
            field_label: None,
            validate_fn: None,
            is_enabled: true,
        }
    }
}

impl FormFieldBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn default<S: Into<String>>(mut self, default: S) -> Self {
        self.default = default.into();
        self
    }

    pub fn validator(
        mut self,
        validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    ) -> Self {
        self.validate_fn = validate_fn;
        self
    }

    pub fn build(self) -> FormElement {
        FormElement::InputField(InputFieldData {
            name: self.name,
            value: self.default,
            field_type: self.field_type,
            field_label: self.field_label,
            validator: self.validate_fn,
            is_enabled: self.is_enabled,
        })
    }

    pub fn field_type(mut self, field_type: FieldType) -> Self {
        self.field_type = field_type;
        self
    }
    pub fn label<S: Into<String>>(mut self, text: S) -> Self {
        self.field_label = Some(FieldLabel::new(&text.into()));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }

    pub fn with_pattern(pattern: InputFieldPattern) -> FormElement {
        match pattern {
            InputFieldPattern::PasswordChange => {
                let password_pattern = Regex::new(r"^.{8,}$").unwrap();
                Self::new("PASSWORD")
                    .default("".to_string())
                    .field_type(FieldType::Password)
                    .validator(Some(Arc::new(validate_with_pattern(
                        password_pattern,
                        "Invalid password. Must be at least 8 characters."
                            .to_string(),
                    ))))
                    .build()
            }
            InputFieldPattern::PasswordCheck => {
                Self::new("PASSWORD").default("".to_string()).field_type(FieldType::Password).build()
            }
        }
    }

    pub fn to_input_field_data(self) -> InputFieldData {
        InputFieldData {
            name: self.name,
            value: self.default,
            field_type: self.field_type,
            field_label: self.field_label,
            validator: self.validate_fn,
            is_enabled: self.is_enabled,
        }
    }

}

pub enum InputFieldPattern {
    PasswordChange,
    PasswordCheck,
}
