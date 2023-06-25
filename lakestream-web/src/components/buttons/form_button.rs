
use super::button_type::ButtonType;

#[derive(Clone)]
pub struct FormButton {
    button_type: ButtonType,
    enabled: bool,
    text: Option<String>,
}

impl FormButton {
    pub fn new(button_type: ButtonType, text: Option<&str>) -> Self {
        Self {
            button_type,
            enabled: true, // default
            text: text.map(|s| s.to_string()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn text(&self) -> String {
        self.text
            .clone()
            .unwrap_or_else(|| self.button_type.button_text().to_string())
    }

    pub fn button_class(&self) -> String {
        self.button_type.button_class(self.enabled)
    }
}

