use std::collections::HashMap;
use std::sync::Arc;

use leptos::*;
use localencrypt::ItemMetaData;

use crate::components::input::{
    DisplayValue, ElementData, ElementDataType, FormElement, FormElementState,
    FormState,
};

pub enum SubmitInput {
    Elements(FormState),
}

#[derive(Clone, Debug)]
pub struct FormData {
    form_state: FormState,
    meta_data: ItemMetaData,
}

impl FormData {
    pub fn new(form_state: FormState, meta_data: ItemMetaData) -> Self {
        Self {
            form_state,
            meta_data,
        }
    }

    pub fn meta_data(&self) -> &ItemMetaData {
        &self.meta_data
    }

    pub fn form_state(&self) -> FormState {
        self.form_state.clone()
    }

    fn create_element_state(
        cx: Scope,
        initial_value: DisplayValue,
        element_data: Arc<ElementData>,
    ) -> FormElementState {
        let error_signal = create_rw_signal(cx, None);
        let value_signal = create_rw_signal(cx, initial_value);

        FormElementState {
            schema: element_data,
            display_value: value_signal,
            display_error: error_signal,
        }
    }

    pub fn update_with_config(&mut self, config: HashMap<String, String>) {
        let is_text_area = self.meta_data()
            .tags()
            .as_ref()
            .and_then(|tags| tags.get("ViewOptions"))
            .map_or(false, |value| value.contains("AsTextArea"));

        if is_text_area {
            // if form is a (single) text area, export config into a set of
            // key=value lines
            let element_name =
                self.form_state.elements().keys().next().unwrap().clone();
            if let Some(form_element_state) =
                self.form_state.elements().clone().get_mut(&element_name)
            {
                // Clone the existing schema to mutate it
                let mut new_schema = (*form_element_state.schema).clone();

                if let ElementDataType::TextData(text_data) =
                    &mut new_schema.element_type
                {
                    if text_data.field_content_type.is_text_area() {
                        // Convert existing TextData to HashMap
                        let mut existing_config: HashMap<String, String> =
                            text_data
                                .buffer_data
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
                        text_data.buffer_data = updated_text_data.clone();
                        form_element_state
                            .display_value
                            .set(DisplayValue::Text(updated_text_data));
                    }
                }

                // Replace old Arc with new one
                form_element_state.schema = Arc::new(new_schema);
            }
        } else {
            // else plot each config item into its own form element
            for (element_name, buffer_data) in config.into_iter() {
                log!("Updating element: {}", element_name);
                if let Some(form_element_state) =
                    self.form_state.elements().clone().get_mut(&element_name)
                {
                    // clone the existing schema to mutate it
                    let mut new_schema = (*form_element_state.schema).clone();
                    match &mut new_schema.element_type {
                        ElementDataType::TextData(text_data) => {
                            text_data.buffer_data = buffer_data.clone();
                            form_element_state
                                .display_value
                                .set(DisplayValue::Text(buffer_data));
                        }
                        ElementDataType::BinaryData(binary_data) => {
                            let binary_buffer_data =
                                buffer_data.as_bytes().to_vec();
                            binary_data.buffer_data =
                                binary_buffer_data.clone();
                            form_element_state
                                .display_value
                                .set(DisplayValue::Binary(binary_buffer_data));
                        }
                        ElementDataType::DocumentData(document_data) => {
                            document_data.buffer_data = buffer_data.clone();
                            form_element_state
                                .display_value
                                .set(DisplayValue::Text(buffer_data));
                        }
                    }
                    // replace old Arc with new one
                    form_element_state.schema = Arc::new(new_schema);
                }
            }
        }
    }

    pub fn build(
        cx: Scope,
        meta_data: ItemMetaData,
        elements: &[FormElement],
    ) -> FormData {
        let elements: HashMap<String, FormElementState> = elements
            .iter()
            .map(|element| {
                let (_name, initial_value) = match element {
                    FormElement::TextBox(data)
                    | FormElement::TextArea(data)
                    | FormElement::NestedForm(data) => {
                        let name = data.name.clone();
                        let initial_value = match &data.element_type {
                            ElementDataType::TextData(text_data) => {
                                DisplayValue::Text(
                                    text_data.buffer_data.clone(),
                                )
                            }
                            ElementDataType::BinaryData(binary_data) => {
                                DisplayValue::Binary(
                                    binary_data.buffer_data.clone(),
                                )
                            }
                            ElementDataType::DocumentData(document_data) => {
                                DisplayValue::Text(
                                    document_data.buffer_data.clone(),
                                )
                            }
                        };
                        (name, initial_value)
                    }
                };
                element.build_form_state(cx, initial_value)
            })
            .collect();
        let form_state = FormState::new(elements);
        Self::new(form_state, meta_data)
    }

    pub fn to_hash_map(&self) -> HashMap<String, String> {
        self.form_state
            .elements()
            .iter()
            .filter_map(|(key, element_state)| {
                match element_state.read_display_value() {
                    DisplayValue::Text(text) => Some((key.clone(), text)),
                    DisplayValue::Binary(_) => None,
                }
            })
            .collect()
    }
}

pub trait FormElementBuilder {
    fn build_form_state(
        &self,
        cx: Scope,
        initial_value: DisplayValue,
    ) -> (String, FormElementState);
}

impl FormElementBuilder for FormElement {
    fn build_form_state(
        &self,
        cx: Scope,
        initial_value: DisplayValue,
    ) -> (String, FormElementState) {
        match self {
            FormElement::TextBox(data)
            | FormElement::TextArea(data)
            | FormElement::NestedForm(data) => {
                let element_state = FormData::create_element_state(
                    cx,
                    initial_value,
                    Arc::new(data.clone()),
                );
                (data.name.clone(), element_state)
            }
        }
    }
}
