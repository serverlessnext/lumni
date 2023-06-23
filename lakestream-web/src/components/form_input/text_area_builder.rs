
use std::sync::Arc;

use super::form_field_builder::{FieldBuilder, FieldBuilderTrait};
use super::{ElementData, ElementDataType, TextData};
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
        let text_data = TextData {
            value: self.default,
            field_type: self.field_type,
            field_label: self.base.field_label(), // assuming you have `field_label` in `TextData`
            validator: self.validate_fn,
        };

        let element_data = ElementData {
            name: self.base.name(),
            element_type: ElementDataType::TextData(text_data),
            is_enabled: self.base.is_enabled(),
            // Add other fields of `ElementData` here if there are any
        };

        FormElement::TextArea(element_data)
    }
}


impl FieldBuilderTrait for TextAreaBuilder {
    fn build(self) -> FormElement {
        TextAreaBuilder::from(self).build()
    }
}

