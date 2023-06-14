use super::field_type::FieldType;

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

#[derive(Clone, Default, Debug)]
pub struct FormField {
    pub field_type: FieldType,
    pub field_label: Option<FieldLabel>,
}
