use std::collections::HashMap;
use std::sync::Arc;

use leptos::*;

use super::form_storage::ConfigurationFormMeta;
use crate::components::forms::input::{
    DisplayValue, FormElement, FormElementState,
};

pub enum SubmitInput {
    Elements(FormElements),
}

pub type FormElements = HashMap<String, FormElementState>;

#[derive(Clone, Debug)]
pub struct FormData {
    elements: FormElements,
    meta_data: ConfigurationFormMeta,
    view_options: FormViewOptions,
}

impl FormData {
    pub fn new(
        elements: FormElements,
        meta_data: ConfigurationFormMeta,
        view_options: Option<FormViewOptions>,
    ) -> Self {
        Self {
            elements,
            meta_data,
            view_options: view_options.unwrap_or_default(),
        }
    }

    pub fn meta_data(&self) -> &ConfigurationFormMeta {
        &self.meta_data
    }

    pub fn elements(&self) -> &FormElements {
        &self.elements
    }

    pub fn update_with_config(&mut self, config: HashMap<String, String>) {
        if self.view_options.text_area() {
            // if form is a (single) text area, export config into a set of
            // key=value lines
            let element_name = self.elements().keys().next().unwrap().clone();
            if let Some(form_element_state) =
                self.elements().clone().get_mut(&element_name)
            {
                // Clone the existing schema to mutate it
                let mut new_schema = (*form_element_state.schema).clone();

                if new_schema.field_content_type.is_text_area() {
                    // Convert existing TextData to HashMap
                    let mut existing_config: HashMap<String, String> =
                        new_schema
                            .buffer_data_as_string()
                            .lines()
                            .filter_map(|line| {
                                let parts: Vec<&str> =
                                    line.splitn(2, '=').collect();
                                if parts.len() == 2 {
                                    Some((
                                        parts[0].trim().to_string(),
                                        parts[1].trim().to_string(),
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect();

                    // Update the HashMap with the provided config
                    existing_config.extend(config);

                    // Convert updated HashMap to String
                    let updated_text_data: String = existing_config
                        .into_iter()
                        .map(|(key, value)| format!("{}={}\n", key, value))
                        .collect();

                    // Update the TextArea with the new string
                    new_schema.buffer_data =
                        updated_text_data.clone().into_bytes();
                    form_element_state
                        .display_value
                        .set(DisplayValue::Text(updated_text_data));
                }

                // Replace old Arc with new one
                form_element_state.schema = Arc::new(new_schema);
            }
        } else {
            // else plot each config item into its own form element
            for (element_name, buffer_data) in config.into_iter() {
                if let Some(form_element_state) =
                    self.elements().clone().get_mut(&element_name)
                {
                    // Clone the existing schema to mutate it
                    let mut new_schema = (*form_element_state.schema).clone();

                    // Update the buffer_data field directly
                    new_schema.buffer_data = buffer_data.clone().into_bytes();

                    // Update the display value
                    form_element_state
                        .display_value
                        .set(DisplayValue::Text(buffer_data));

                    // Replace old Arc with new one
                    form_element_state.schema = Arc::new(new_schema);
                }
            }
        }
    }

    pub fn export_config(&self) -> HashMap<String, String> {
        if self.view_options.text_area() {
            self.elements()
                .values()
                .next()
                .unwrap()
                .read_display_value()
                .as_text()
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        Some((
                            parts[0].trim().to_string(),
                            parts[1].trim().to_string(),
                        ))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            self.elements()
                .iter()
                .map(|(key, element_state)| {
                    (
                        key.clone(),
                        match element_state.read_display_value() {
                            DisplayValue::Text(text) => text,
                        },
                    )
                })
                .collect()
        }
    }

    pub fn build(
        cx: Scope,
        meta_data: ConfigurationFormMeta,
        elements: &[FormElement],
        view_options: Option<FormViewOptions>,
    ) -> FormData {
        let elements: HashMap<String, FormElementState> = elements
            .iter()
            .map(|element| {
                let name = element.name.clone();
                let initial_value =
                    DisplayValue::Text(element.buffer_data_as_string());
                let error_signal = create_rw_signal(cx, None);
                let value_signal = create_rw_signal(cx, initial_value);

                let element_state = FormElementState {
                    schema: Arc::new(element.clone()),
                    display_value: value_signal,
                    display_error: error_signal,
                };

                (name, element_state)
            })
            .collect();
        Self::new(elements, meta_data, view_options)
    }
}

#[derive(Clone, Debug)]
pub struct FormViewOptions {
    text_area: bool,
}

impl Default for FormViewOptions {
    fn default() -> Self {
        Self { text_area: false }
    }
}

impl FormViewOptions {
    pub fn text_area(&self) -> bool {
        self.text_area
    }

    pub fn set_text_area(&mut self, text_area: bool) {
        self.text_area = text_area;
    }
}
