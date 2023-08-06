

use std::collections::HashMap;
use leptos::*;

use crate::components::form_input::FieldType;
use crate::components::forms::Form;

use super::form_builder::{FormBuilder, FormType};
use super::ElementBuilder;


pub struct ProfileFormBuilder {
    inner: FormBuilder,
}

impl ProfileFormBuilder {
    pub fn new<S: Into<String>>(
        title: S,
        id: S,
        tags: Option<HashMap<String, String>>,
        form_type: FormType,
    ) -> Self {
        Self {
            inner: FormBuilder::new(title, id, tags, form_type),
        }
    }

    pub fn with_elements<I, T>(mut self, form_elements: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<ElementBuilder>,
    {
        self.inner = self.inner.with_elements(form_elements);
        self
    }

    pub fn build(self, cx: Scope) -> Box<dyn Form> {
        self.inner.build(cx)
    }

    pub fn to_text_area(mut self) -> FormBuilder {
        let mut text_area_content = String::new();

        for element in self.inner.get_elements() {
            let key = element.name();
            let value = element.get_initial_value();
            text_area_content.push_str(&format!("{}={}\n", key, value));
        }

        self.inner.clear_elements();
        self.inner.add_element(
            ElementBuilder::new("FORM_CONTENT", FieldType::TextArea)
                .with_label("Form Content")
                .with_initial_value(text_area_content),
        );

        self.inner
    }
}

