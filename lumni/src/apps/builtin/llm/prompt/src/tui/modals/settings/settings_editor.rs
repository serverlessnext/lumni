use serde_json::{Map, Value as JsonValue};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsAction {
    ToggleSecureVisibility,
    DeleteCurrentKey,
    ClearCurrentKey,
    SaveEdit,
    SaveNewValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsEditor {
    settings: JsonValue,
    current_field: usize,
    edit_buffer: String,
    new_key_buffer: String,
    is_new_value_secure: bool,
    pub show_secure: bool,
    pub edit_mode: EditMode,
}

impl SettingsEditor {
    pub fn new(settings: JsonValue) -> Self {
        Self {
            settings,
            current_field: 0,
            edit_buffer: String::new(),
            new_key_buffer: String::new(),
            is_new_value_secure: false,
            show_secure: false,
            edit_mode: EditMode::NotEditing,
        }
    }

    pub fn load_settings(&mut self, settings: JsonValue) {
        self.settings = settings;
        self.current_field = 0;
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    pub fn handle_key_event(
        &mut self,
        key_code: KeyCode,
    ) -> (EditMode, bool, Option<SettingsAction>) {
        match self.edit_mode {
            EditMode::NotEditing => match key_code {
                KeyCode::Up => {
                    self.move_selection_up();
                    (EditMode::NotEditing, true, None)
                }
                KeyCode::Down => {
                    self.move_selection_down();
                    (EditMode::NotEditing, true, None)
                }
                KeyCode::Enter => {
                    if self.start_editing().is_some() {
                        (EditMode::EditingValue, true, None)
                    } else {
                        (EditMode::NotEditing, false, None)
                    }
                }
                KeyCode::Char('n') => {
                    self.start_adding_new_value(false);
                    (EditMode::AddingNewKey, true, None)
                }
                KeyCode::Char('N') => {
                    self.start_adding_new_value(true);
                    (EditMode::AddingNewKey, true, None)
                }
                KeyCode::Char('s') | KeyCode::Char('S') => (
                    EditMode::NotEditing,
                    true,
                    Some(SettingsAction::ToggleSecureVisibility),
                ),
                KeyCode::Char('D') => (
                    EditMode::NotEditing,
                    true,
                    Some(SettingsAction::DeleteCurrentKey),
                ),
                KeyCode::Char('C') => (
                    EditMode::NotEditing,
                    true,
                    Some(SettingsAction::ClearCurrentKey),
                ),
                _ => (EditMode::NotEditing, false, None),
            },
            EditMode::EditingValue => match key_code {
                KeyCode::Enter => {
                    (EditMode::NotEditing, true, Some(SettingsAction::SaveEdit))
                }
                KeyCode::Esc => {
                    self.cancel_edit();
                    (EditMode::NotEditing, true, None)
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                    (EditMode::EditingValue, true, None)
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                    (EditMode::EditingValue, true, None)
                }
                _ => (EditMode::EditingValue, false, None),
            },
            EditMode::AddingNewKey => match key_code {
                KeyCode::Enter => {
                    if self.confirm_new_key() {
                        (EditMode::AddingNewValue, true, None)
                    } else {
                        (EditMode::AddingNewKey, false, None)
                    }
                }
                KeyCode::Esc => {
                    self.cancel_edit();
                    (EditMode::NotEditing, true, None)
                }
                KeyCode::Backspace => {
                    self.new_key_buffer.pop();
                    (EditMode::AddingNewKey, true, None)
                }
                KeyCode::Char(c) => {
                    self.new_key_buffer.push(c);
                    (EditMode::AddingNewKey, true, None)
                }
                _ => (EditMode::AddingNewKey, false, None),
            },
            EditMode::AddingNewValue => match key_code {
                KeyCode::Enter => (
                    EditMode::NotEditing,
                    true,
                    Some(SettingsAction::SaveNewValue),
                ),
                KeyCode::Esc => {
                    self.cancel_edit();
                    (EditMode::NotEditing, true, None)
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                    (EditMode::AddingNewValue, true, None)
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                    (EditMode::AddingNewValue, true, None)
                }
                _ => (EditMode::AddingNewValue, false, None),
            },
        }
    }

    pub fn get_display_value(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::Object(obj)
                if obj.get("was_encrypted") == Some(&JsonValue::Bool(true)) =>
            {
                let display = if self.show_secure {
                    match obj.get("content") {
                        Some(JsonValue::String(s)) => s.clone(),
                        _ => "Invalid Value".to_string(),
                    }
                } else {
                    "*****".to_string()
                };
                format!("{} (Encrypted)", display)
            }
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "null".to_string(),
            _ => value.to_string(),
        }
    }

    pub fn start_adding_new_value(&mut self, is_secure: bool) {
        self.new_key_buffer.clear();
        self.edit_buffer.clear();
        self.is_new_value_secure = is_secure;
        self.edit_mode = EditMode::AddingNewKey;
    }

    pub fn get_settings(&self) -> &JsonValue {
        &self.settings
    }

    pub fn get_current_field(&self) -> usize {
        self.current_field
    }

    pub fn get_edit_buffer(&self) -> &str {
        &self.edit_buffer
    }

    pub fn get_new_key_buffer(&self) -> &str {
        &self.new_key_buffer
    }

    pub fn is_new_value_secure(&self) -> bool {
        self.is_new_value_secure
    }

    fn move_selection_up(&mut self) {
        if self.current_field > 0 {
            self.current_field -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        let settings_len = self.settings.as_object().map_or(0, |obj| obj.len());
        if settings_len > 0 && self.current_field < settings_len - 1 {
            self.current_field += 1;
        }
    }

    fn start_editing(&mut self) -> Option<String> {
        let current_key = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field);

        if let Some(key) = current_key {
            if !key.starts_with("__") {
                let value = &self.settings[key];
                self.edit_buffer = match value {
                    JsonValue::Object(obj)
                        if obj.get("was_encrypted")
                            == Some(&JsonValue::Bool(true)) =>
                    {
                        match obj.get("content") {
                            Some(JsonValue::String(s)) => s.clone(),
                            _ => String::new(),
                        }
                    }
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                Some(self.edit_buffer.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn confirm_new_key(&mut self) -> bool {
        !self.new_key_buffer.is_empty()
    }

    fn cancel_edit(&mut self) {
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    pub fn get_current_key(&self) -> Option<&str> {
        self.settings
            .as_object()
            .and_then(|obj| obj.keys().nth(self.current_field))
            .map(String::as_str)
    }

    pub fn clear(&mut self) {
        self.settings = JsonValue::Object(Map::new());
        self.current_field = 0;
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }
}

pub trait SettingsItem {
    fn name(&self) -> &str;
    fn item_type(&self) -> &'static str;
}

impl SettingsItem for UserProfile {
    fn name(&self) -> &str {
        &self.name
    }

    fn item_type(&self) -> &'static str {
        "Profile"
    }
}

impl SettingsItem for DatabaseConfigurationItem {
    fn name(&self) -> &str {
        &self.name
    }

    fn item_type(&self) -> &'static str {
        "Configuration"
    }
}
