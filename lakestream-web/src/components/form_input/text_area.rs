
use std::fmt;
use std::sync::Arc;

use super::FieldLabel;


#[derive(Debug, Clone)]
pub struct TextArea {
    pub name: String,
    pub text_area_data: TextAreaData,
}

#[derive(Clone)]
pub struct TextAreaData {
    pub name: String,
    pub value: String,
    pub field_label: Option<FieldLabel>,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    pub is_enabled: bool,
}

impl fmt::Debug for TextAreaData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextAreaData")
            .field("name", &self.name)
            .field("value", &self.value)
            .field("field_label", &self.field_label)
            .field("validator", &self.validator.is_some())
            .field("is_enabled", &self.is_enabled)
            .finish()
    }
}

