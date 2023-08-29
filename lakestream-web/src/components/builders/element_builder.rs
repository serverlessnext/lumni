use std::fmt;
use std::sync::Arc;

use regex::Regex;

use crate::components::input::{
    validate_with_pattern, FieldContentType, FieldLabel, FieldPlaceholder,
    FormElement,
};

#[derive(Clone)]
pub struct ElementBuilder {
    name: String,
    field_label: Option<FieldLabel>,
    is_enabled: bool,
    initial_value: String,
    field_placeholder: Option<FieldPlaceholder>,
    field_content_type: FieldContentType,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl fmt::Debug for ElementBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ElementBuilder")
            .field("name", &self.name)
            .field("field_label", &self.field_label)
            .field("is_enabled", &self.is_enabled)
            .field("initial_value", &self.initial_value)
            .field("field_placeholder", &self.field_placeholder)
            .field("field_content_type", &self.field_content_type)
            .field("validate_fn", &self.validate_fn.is_some()) // Displaying only if the function exists
            .finish()
    }
}

impl ElementBuilder {
    pub fn new<S: Into<String>>(
        name: S,
        field_content_type: FieldContentType,
    ) -> Self {
        Self {
            name: name.into(),
            field_label: None,
            is_enabled: true,
            initial_value: String::new(),
            field_placeholder: None,
            field_content_type,
            validate_fn: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn validate_fn(
        &self,
    ) -> Option<Arc<dyn Fn(&str) -> Result<(), String>>> {
        self.validate_fn.clone()
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

    pub fn with_placeholder<S: Into<String>>(mut self, placeholder: S) -> Self {
        self.field_placeholder =
            Some(FieldPlaceholder::new(placeholder.into()));
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
            InputFieldPattern::PasswordCheck => {
                ElementBuilder::new("PASSWORD", FieldContentType::Password)
                    .with_label("Password")
            }
        }
    }

    pub fn build(self) -> FormElement {
        FormElement {
            name: self.name,
            field_content_type: self.field_content_type.clone(),
            field_label: self.field_label,
            field_placeholder: self.field_placeholder,
            validator: self.validate_fn,
            buffer_data: self.initial_value.into_bytes(),
            is_enabled: self.is_enabled,
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
