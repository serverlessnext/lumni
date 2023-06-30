use std::sync::Arc;

use regex::Regex;

use super::field_builder::{FieldBuilder, FieldBuilderTrait};
use crate::components::form_input::{
    validate_with_pattern, ElementData, ElementDataType, FieldType,
    FormElement, TextData,
};

type ValidateFn = Arc<dyn Fn(&str) -> Result<(), String>>;

pub struct TextBoxBuilder {
    base: FieldBuilder,
    initial_value: String,
    field_type: FieldType,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl Clone for TextBoxBuilder {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            initial_value: self.initial_value.clone(),
            field_type: self.field_type.clone(),
            validate_fn: self.validate_fn.clone(),
        }
    }
}

impl From<FieldBuilder> for TextBoxBuilder {
    fn from(field_builder: FieldBuilder) -> Self {
        Self {
            base: field_builder,
            initial_value: String::new(),
            field_type: FieldType::Text,
            validate_fn: None,
        }
    }
}

impl TextBoxBuilder {
    pub fn new(
        base: FieldBuilder,
        initial_value: String,
        field_type: FieldType,
        validate_fn: Option<ValidateFn>,
    ) -> Self {
        Self {
            base,
            initial_value,
            field_type,
            validate_fn,
        }
    }

    pub fn with_initial_value<S: Into<String>>(mut self, value: S) -> Self {
        self.initial_value = value.into();
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
                FieldBuilder::new("PASSWORD")
                    .with_label("Password")
                    .as_input_field()
                    .field_type(FieldType::Password)
                    .validator(Some(Arc::new(validate_with_pattern(
                        password_pattern,
                        "Invalid password. Must be at least 8 characters."
                            .to_string(),
                    ))))
            }
            InputFieldPattern::PasswordCheck => FieldBuilder::new("PASSWORD")
                .with_label("Password")
                .as_input_field()
                .field_type(FieldType::Password),
        }
    }
    pub fn build(self) -> FormElement {
        let text_data = TextData {
            field_label: self.base.field_label(),
            field_type: self.field_type,
            validator: self.validate_fn,
            buffer_data: self.initial_value,
        };

        FormElement::TextBox(ElementData {
            name: self.base.name(),
            element_type: ElementDataType::TextData(text_data),
            is_enabled: self.base.is_enabled(),
        })
    }
}

impl FieldBuilderTrait for TextBoxBuilder {
    fn build(&self) -> FormElement {
        self.clone().build()
    }

    fn box_clone(&self) -> Box<dyn FieldBuilderTrait> {
        Box::new(self.clone())
    }
}

pub enum InputFieldPattern {
    PasswordChange,
    PasswordCheck,
}
