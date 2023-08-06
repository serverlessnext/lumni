use std::sync::Arc;
use regex::Regex;

use crate::components::form_input::{
    validate_with_pattern, ElementData, ElementDataType, FieldContentType,
    FieldLabel, FormElement, TextData,
};

#[derive(Clone)]
pub struct ElementBuilder {
    name: String,
    field_label: Option<FieldLabel>,
    is_enabled: bool,
    initial_value: String,
    field_content_type: FieldContentType,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl ElementBuilder {
    pub fn new<S: Into<String>>(name: S, field_content_type: FieldContentType) -> Self {
        Self {
            name: name.into(),
            field_label: None,
            is_enabled: true,
            initial_value: String::new(),
            field_content_type,
            validate_fn: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn with_label<S: Into<String>>(mut self, label: S) -> Self {
        self.field_label = Some(FieldLabel::new(label.into()));
        self
    }

    pub fn get_initial_value(&self) -> &str {
        &self.initial_value
    }

    pub fn with_initial_value<S: Into<String>>(mut self, value: S) -> Self {
        self.initial_value = value.into();
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
                ElementBuilder::new("PASSWORD", FieldContentType::Password)
                    .with_label("Password")
                    .validator(Some(Arc::new(validate_with_pattern(
                        password_pattern,
                        "Invalid password. Must be at least 8 characters."
                            .to_string(),
                    ))))
            }
            InputFieldPattern::PasswordCheck => ElementBuilder::new("PASSWORD", FieldContentType::Password)
                .with_label("Password")
        }
    }

    pub fn build(self) -> FormElement {
        let text_data = TextData {
            field_label: self.field_label,
            field_content_type: self.field_content_type.clone(),
            validator: self.validate_fn,
            buffer_data: self.initial_value,
        };

        let element_data = ElementData {
            name: self.name,
            element_type: ElementDataType::TextData(text_data),
            is_enabled: self.is_enabled,
        };

        match self.field_content_type {
            FieldContentType::PlainText | FieldContentType::Secret | FieldContentType::Password => FormElement::TextBox(element_data),
            // FieldType::TextArea => FormElement::TextArea(element_data),
        }
    }
}

impl FieldBuilderTrait for ElementBuilder {
    fn build(&self) -> FormElement {
        self.clone().build()
    }

    fn box_clone(&self) -> Box<dyn FieldBuilderTrait> {
        Box::new(self.clone())
    }
}

pub trait FieldBuilderTrait {
    fn build(&self) -> FormElement;
    fn box_clone(&self) -> Box<dyn FieldBuilderTrait>;
}

pub fn build_all<T: FieldBuilderTrait>(builders: Vec<T>) -> Vec<FormElement> {
    builders
        .into_iter()
        .map(|builder| builder.build())
        .collect()
}

pub enum InputFieldPattern {
    PasswordChange,
    PasswordCheck,
}
