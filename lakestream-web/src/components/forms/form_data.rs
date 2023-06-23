use std::collections::HashMap;
use std::sync::Arc;

use leptos::*;
use localencrypt::ItemMetaData;

use crate::components::form_input::{FormElement, InputElements};

pub enum SubmitInput {
    Elements(InputElements),
}

#[derive(Clone)]
pub struct FormData {
    input_elements: InputElements,
    meta_data: ItemMetaData,
}

impl FormData {
    pub fn new(input_elements: InputElements, meta_data: ItemMetaData) -> Self {
        Self {
            input_elements,
            meta_data,
        }
    }

    pub fn meta_data(&self) -> &ItemMetaData {
        &self.meta_data
    }

    pub fn input_elements(&self) -> InputElements {
        self.input_elements.clone()
    }

    pub fn create_from_elements(
        cx: Scope,
        meta_data: ItemMetaData,
        config: &HashMap<String, String>,
        elements: &[FormElement],
    ) -> FormData {
        let input_elements: InputElements = config
            .iter()
            .filter_map(|(key, value)| {
                elements.iter().find_map(|element| match element {
                    FormElement::TextBox(field_data) => {
                        if field_data.name == *key {
                            let error_signal = create_rw_signal(cx, None);
                            let value_signal = create_rw_signal(cx, value.clone());
                            let default_input_data = field_data.clone();
                            Some((
                                key.clone(),
                                (
                                    create_node_ref(cx),
                                    error_signal,
                                    value_signal,
                                    Arc::new(default_input_data),
                                ),
                            ))
                        } else {
                            None
                        }
                    },
                    FormElement::TextArea(field_data) => {
                        // TODO: implement this
                        None
                    }
                })

            })
            .collect();
        Self::new(input_elements, meta_data)
    }

    pub fn to_hash_map(&self) -> HashMap<String, String> {
        let document_content: HashMap<String, String> = self
            .input_elements
            .iter()
            .map(|(key, (_, _, value_signal, _))| {
                (key.clone(), value_signal.get())
            })
            .collect();
        document_content
    }
}
