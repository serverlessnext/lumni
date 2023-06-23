
use std::sync::Arc;
use regex::Regex;

use super::form_field_builder::{FieldBuilder, FieldBuilderTrait};
use super::helpers::validate_with_pattern;
use super::text_box::InputFieldData;
use crate::components::form_input::{FieldType, FormElement};


pub struct TextBoxBuilder {
    base: FieldBuilder,
    default: String,
    field_type: FieldType,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl From<FieldBuilder> for TextBoxBuilder {
    fn from(field_builder: FieldBuilder) -> Self {
        Self {
            base: field_builder,
            default: String::new(),
            field_type: FieldType::Text,
            validate_fn: None,
        }
    }
}

impl TextBoxBuilder {
    pub fn new(base: FieldBuilder, default: String, field_type: FieldType, validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>) -> Self {
        Self {
            base,
            default,
            field_type,
            validate_fn,
        }
    }

    pub fn default<S: Into<String>>(mut self, default: S) -> Self {
        self.default = default.into();
        self
    }

    pub fn field_type(mut self, field_type: FieldType) -> Self {
        self.field_type = field_type;
        self
    }

    pub fn validator(
        mut self,
        validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    ) -> Self {
        self.validate_fn = validate_fn;
        self
    }

    pub fn with_pattern(pattern: InputFieldPattern) -> Self {
        match pattern {
            InputFieldPattern::PasswordChange => {
                let password_pattern = Regex::new(r"^.{8,}$").unwrap();
                TextBoxBuilder::from(
                    FieldBuilder::new("PASSWORD")
                        .label("Password")
                        .as_input_field(),
                )
                .default("".to_string())
                .field_type(FieldType::Password)
                .validator(Some(Arc::new(
                    validate_with_pattern(
                        password_pattern,
                        "Invalid password. Must be at least 8 characters."
                            .to_string(),
                    ),
                )))
            }
            InputFieldPattern::PasswordCheck => TextBoxBuilder::from(
                FieldBuilder::new("PASSWORD")
                    .label("Password")
                    .as_input_field(),
            )
            .default("".to_string())
            .field_type(FieldType::Password),
        }
    }

    pub fn build(self) -> FormElement {
        FormElement::TextBox(InputFieldData {
            name: self.base.name(),
            value: self.default,
            field_type: self.field_type,
            field_label: self.base.field_label(),
            validator: self.validate_fn,
            is_enabled: self.base.is_enabled(),
        })
    }
}

impl FieldBuilderTrait for TextBoxBuilder {
    fn build(self) -> FormElement {
        TextBoxBuilder::from(self).build()
    }
}

pub enum InputFieldPattern {
    PasswordChange,
    PasswordCheck,
}
