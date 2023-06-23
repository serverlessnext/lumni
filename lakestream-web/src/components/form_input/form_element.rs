use super::text_box::InputFieldData;

#[derive(Clone, Default, Debug)]
pub struct FieldLabel {
    text: String,
}

impl FieldLabel {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }
}

#[derive(Debug, Clone)]
pub enum FormElement {
    TextBox(InputFieldData),
    TextArea(InputFieldData),
}
