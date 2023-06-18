
use super::InputFieldData;


#[derive(Clone, Default, Debug)]
pub struct FieldLabel {
    text: String,
}

impl FieldLabel {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone)]
pub enum FormElement {
    InputField(InputFieldData),
}

