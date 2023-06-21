use localencrypt::ItemMetaData;

use crate::components::form_input::InputElements;

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
}
