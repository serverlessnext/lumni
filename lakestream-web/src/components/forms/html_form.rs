use std::collections::HashMap;

use crate::components::form_input::{ElementDataType, FormElement};

#[derive(Clone, Debug)]
pub struct HtmlForm {
    name: String,
    id: String,
    elements: Vec<FormElement>,
}

impl HtmlForm {
    pub fn new(name: &str, id: &str, elements: Vec<FormElement>) -> Self {
        Self {
            name: name.to_string(),
            id: id.to_string(),
            elements,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn elements(&self) -> Vec<FormElement> {
        self.elements.clone()
    }

    pub fn default_field_values(&self) -> HashMap<String, String> {
        self.elements
            .iter()
            .filter_map(|element| match element {
                FormElement::TextBox(element_data)
                | FormElement::TextArea(element_data) => {
                    if let ElementDataType::TextData(text_data) =
                        &element_data.element_type
                    {
                        Some((
                            element_data.name.clone(),
                            text_data.buffer_data.clone(),
                        ))
                    } else {
                        None
                    }
                }
                FormElement::NestedForm(element_data) => {
                    if let ElementDataType::DocumentData(nested_form_data) =
                        &element_data.element_type
                    {
                        Some((
                            element_data.name.clone(),
                            nested_form_data.buffer_data.clone(),
                        ))
                    } else {
                        None
                    }
                }
            })
            .collect()
    }
}
