use std::sync::Arc;

use regex::Regex;

use super::helpers::validate_with_pattern;
use super::InputFieldData;
use crate::components::form_input::{FieldLabel, FieldType, FormElement};

#[derive(Clone)]
pub struct FieldBuilder {
    name: String,
    field_label: Option<FieldLabel>,
    is_enabled: bool,
}

impl FieldBuilder {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            field_label: None,
            is_enabled: true,
        }
    }

    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.field_label = Some(FieldLabel::new(label.into()));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }

    // transition methods for specific form elements
    pub fn as_input_field(self) -> InputFieldBuilder {
        InputFieldBuilder {
            base: self,
            default: String::new(),
            field_type: FieldType::Text,
            validate_fn: None,
        }
    }
}

pub struct InputFieldBuilder {
    base: FieldBuilder,
    default: String,
    field_type: FieldType,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl From<FieldBuilder> for InputFieldBuilder {
    fn from(field_builder: FieldBuilder) -> Self {
        Self {
            base: field_builder,
            default: String::new(),
            field_type: FieldType::Text,
            validate_fn: None,
        }
    }
}

impl InputFieldBuilder {
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
                InputFieldBuilder::from(
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
            InputFieldPattern::PasswordCheck => InputFieldBuilder::from(
                FieldBuilder::new("PASSWORD")
                    .label("Password")
                    .as_input_field(),
            )
            .default("".to_string())
            .field_type(FieldType::Password),
        }
    }

    pub fn build(self) -> FormElement {
        FormElement::InputField(InputFieldData {
            name: self.base.name,
            value: self.default,
            field_type: self.field_type,
            field_label: self.base.field_label,
            validator: self.validate_fn,
            is_enabled: self.base.is_enabled,
        })
    }
}

impl FieldBuilderTrait for InputFieldBuilder {
    fn build(self) -> FormElement {
        InputFieldBuilder::from(self).build()
    }
}

pub trait FieldBuilderTrait {
    fn build(self) -> FormElement;
}

pub enum InputFieldPattern {
    PasswordChange,
    PasswordCheck,
}

pub fn build_all<T: FieldBuilderTrait>(builders: Vec<T>) -> Vec<FormElement> {
    builders
        .into_iter()
        .map(|builder| builder.build())
        .collect()
}
