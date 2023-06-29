use std::collections::HashMap;
use std::sync::Arc;

use leptos::*;
use localencrypt::ItemMetaData;

use crate::components::form_input::{
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

    pub fn build_with_config(
        cx: Scope,
        meta_data: ItemMetaData,
        config: &HashMap<String, String>,
        elements: &[FormElement],
    ) -> FormData {
        let form_state: FormState = config
            .iter()
            .filter_map(|(key, value)| {
                elements.iter().find_map(|element| match element {
                    FormElement::TextBox(field_data)
                    | FormElement::TextArea(field_data)
                    | FormElement::NestedForm(field_data) => {
                        if field_data.name == *key {
                            let initial_value =
                                DisplayValue::Text(value.clone());
                            Some(element.build_form_state(cx, initial_value))
                        } else {
                            None
                        }
                    }
                })
            })
            .collect();
        Self::new(form_state, meta_data)
    }

    pub fn update_with_config(&mut self, config: HashMap<String, String>) {
        for (element_name, buffer_data) in config.into_iter() {
            if let Some(form_element_state) = self.form_state.get_mut(&element_name) {
                // clone the existing schema to mutate it
                let mut new_schema = (*form_element_state.schema).clone();
                match &mut new_schema.element_type {
                    ElementDataType::TextData(text_data) => {
                        text_data.buffer_data = buffer_data.clone();
                        form_element_state.display_value.set(DisplayValue::Text(buffer_data));
                    }
                    ElementDataType::BinaryData(binary_data) => {
                        let binary_buffer_data = buffer_data.as_bytes().to_vec();
                        binary_data.buffer_data = binary_buffer_data.clone();
                        form_element_state.display_value.set(DisplayValue::Binary(binary_buffer_data));
                    }
                    ElementDataType::DocumentData(document_data) => {
                        document_data.buffer_data = buffer_data.clone();
                        form_element_state.display_value.set(DisplayValue::Text(buffer_data));
                    }
                }
                // replace old Arc with new one
                form_element_state.schema = Arc::new(new_schema);
            }
        }
    }

    pub fn build(
        cx: Scope,
        meta_data: ItemMetaData,
        elements: &[FormElement],
    ) -> FormData {
        let form_state: FormState = elements
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

        Self::new(form_state, meta_data)
    }

    pub fn to_hash_map(&self) -> HashMap<String, String> {
        self.form_state
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
