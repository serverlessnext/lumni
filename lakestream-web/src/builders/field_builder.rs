use super::text_box_builder::TextBoxBuilder;
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

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn field_label(&self) -> Option<FieldLabel> {
        self.field_label.clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    pub fn with_label<S: Into<String>>(mut self, label: S) -> Self {
        self.field_label = Some(FieldLabel::new(label.into()));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }

    // transition methods for specific form elements
    pub fn as_input_field(self) -> TextBoxBuilder {
        TextBoxBuilder::new(self, String::new(), FieldType::Text, None)
    }
}

pub trait FieldBuilderTrait {
    fn build(&self) -> FormElement;
}

pub fn build_all<T: FieldBuilderTrait>(builders: Vec<T>) -> Vec<FormElement> {
    builders
        .into_iter()
        .map(|builder| builder.build())
        .collect()
}
