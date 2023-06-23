
use std::sync::Arc;

use super::form_field_builder::{FieldBuilder, FieldBuilderTrait};
use super::text_box::InputFieldData;
use crate::components::form_input::{FieldType, FormElement};


pub struct TextAreaBuilder {
    base: FieldBuilder,
    default: String,
    field_type: FieldType,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl From<FieldBuilder> for TextAreaBuilder {
    fn from(field_builder: FieldBuilder) -> Self {
        Self {
            base: field_builder,
            default: String::new(),
            field_type: FieldType::Text,
            validate_fn: None,
        }
    }
}

impl TextAreaBuilder {
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

impl FieldBuilderTrait for TextAreaBuilder {
    fn build(self) -> FormElement {
        TextAreaBuilder::from(self).build()
    }
}

