use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::form_builder::{FormBuilder, FormType};
use super::ElementBuilder;
use crate::components::forms::input::FieldContentType;
use crate::components::forms::{ConfigurationFormMeta, Form, FormViewOptions};

pub struct ProfileFormBuilder {
    inner: FormBuilder,
    view_options: FormViewOptions,
}

impl ProfileFormBuilder {
    pub fn new<S: Into<String>>(
        title: S,
        form_meta: ConfigurationFormMeta,
        form_type: FormType,
    ) -> Self {
        Self {
            inner: FormBuilder::new(title, form_meta, form_type),
            view_options: FormViewOptions::default(),
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

    pub fn build(self) -> Box<dyn Form> {
        self.inner.build(Some(self.view_options))
    }

    pub fn to_text_area(mut self) -> ProfileFormBuilder {
        let mut text_area_content = String::new();
        let mut original_validators = HashMap::new();
        let mut expected_keys: HashSet<String> = HashSet::new();

        // Extract the original validators and build the text area content
        for element in self.inner.get_elements() {
            let key = element.name();
            let value = element.get_initial_value();
            text_area_content.push_str(&format!("{}={}\n", key, value));

            if let Some(validator) = element.validate_fn() {
                original_validators.insert(key.to_string(), validator);
            }

            expected_keys.insert(key.to_string());
        }

        // Create a new validation function
        let new_validator =
            Arc::new(move |text_area_content: &str| -> Result<(), String> {
                let mut found_keys: HashSet<String> = HashSet::new();
                for line in text_area_content.lines() {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() != 2 {
                        return Err(format!("Invalid line: {}", line));
                    }
                    let key = parts[0].trim().to_string();
                    let value = parts[1].trim();

                    if let Some(validator) = original_validators.get(&key) {
                        validator(value)?;
                    }

                    found_keys.insert(key);
                }

                // Check if there are any missing keys
                let missing_keys: HashSet<_> =
                    expected_keys.difference(&found_keys).collect();
                if !missing_keys.is_empty() {
                    return Err(format!("Missing keys: {:?}", missing_keys));
                }

                Ok(())
            });

        // Clear the existing elements and add the new text area element
        self.inner.clear_elements();
        self.inner.add_element(
            ElementBuilder::new("FORM_CONTENT", FieldContentType::TextArea)
                .with_label("Form Content")
                .with_initial_value(text_area_content)
                .validator(Some(new_validator)),
        );

        self.view_options.set_text_area(true);
        self
    }
}
