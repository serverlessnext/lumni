use std::collections::HashMap;
use std::sync::Arc;

use leptos::*;
use localencrypt::ItemMetaData;

use crate::components::form_input::{
    DisplayValue, FormElement, FormElementState, FormState,
};

pub enum SubmitInput {
    Elements(FormState),
}

#[derive(Clone)]
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

    pub fn create_from_elements(
        cx: Scope,
        meta_data: ItemMetaData,
        config: &HashMap<String, String>,
        elements: &[FormElement],
    ) -> FormData {
        let form_state: FormState = config
            .iter()
            .filter_map(|(key, value)| {
                elements.iter().find_map(|element| match element {
                    FormElement::TextBox(field_data) => {
                        if field_data.name == *key {
                            let error_signal = create_rw_signal(cx, None);
                            let value_signal = create_rw_signal(
                                cx,
                                DisplayValue::Text(value.clone()),
                            );
                            let default_input_data =
                                Arc::new(field_data.clone());
                            Some((
                                key.clone(),
                                FormElementState {
                                    schema: default_input_data,
                                    display_value: value_signal,
                                    display_error: error_signal,
                                },
                            ))
                        } else {
                            None
                        }
                    }
                    FormElement::TextArea(_field_data) => {
                        panic!("TextArea not implemented yet")
                    }

                    FormElement::NestedForm(field_data) => {
                        if field_data.name == *key {
                            let error_signal = create_rw_signal(cx, None);
                            let value_signal = create_rw_signal(
                                cx,
                                DisplayValue::Text(value.clone()),
                            );
                            let default_input_data =
                                Arc::new(field_data.clone());
                            Some((
                                key.clone(),
                                FormElementState {
                                    schema: default_input_data,
                                    display_value: value_signal,
                                    display_error: error_signal,
                                },
                            ))
                        } else {
                            None
                        }
                    }
                })
            })
            .collect();
        Self::new(form_state, meta_data)
    }

    pub fn to_hash_map(&self) -> HashMap<String, String> {
        self.form_state
            .iter()
            .filter_map(|(key, element_state)| {
                match element_state.display_value.get_untracked() {
                    DisplayValue::Text(text) => Some((key.clone(), text)),
                    DisplayValue::Binary(_) => None,
                }
            })
            .collect()
    }
}
