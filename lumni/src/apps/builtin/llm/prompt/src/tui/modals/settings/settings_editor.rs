use serde_json::{Map, Value as JsonValue};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsAction {
    ToggleSecureVisibility,
    DeleteCurrentKey,
    ClearCurrentKey,
    SaveEdit,
    SaveNewValue,
    OpenSection,
    CloseSection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsEditor {
    settings: JsonValue,
    pub current_path: Vec<String>,
    current_field: String,
    edit_buffer: String,
    new_key_buffer: String,
    is_new_value_secure: bool,
    pub show_secure: bool,
    pub edit_mode: EditMode,
}

impl SettingsEditor {
    pub fn new(settings: JsonValue) -> Self {
        let current_field = settings
            .as_object()
            .and_then(|obj| obj.keys().next().cloned())
            .unwrap_or_default();

        Self {
            settings,
            current_path: Vec::new(),
            current_field,
            edit_buffer: String::new(),
            new_key_buffer: String::new(),
            is_new_value_secure: false,
            show_secure: false,
            edit_mode: EditMode::NotEditing,
        }
    }

    pub fn load_settings(&mut self, settings: JsonValue) {
        self.settings = settings;
        self.current_path.clear();
        self.current_field = self
            .settings
            .as_object()
            .and_then(|obj| obj.keys().next().cloned())
            .unwrap_or_default();
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    pub fn handle_key_event(
        &mut self,
        key: KeyCode,
    ) -> (EditMode, bool, Option<SettingsAction>) {
        match self.edit_mode {
            EditMode::NotEditing => self.handle_not_editing(key),
            EditMode::EditingValue => self.handle_editing_value(key),
            EditMode::AddingNewKey => self.handle_adding_new_key(key),
            EditMode::AddingNewValue => self.handle_adding_new_value(key),
        }
    }

    fn handle_not_editing(
        &mut self,
        key: KeyCode,
    ) -> (EditMode, bool, Option<SettingsAction>) {
        match key {
            KeyCode::Up => {
                self.move_selection(-1);
                (EditMode::NotEditing, true, None)
            }
            KeyCode::Down => {
                self.move_selection(1);
                (EditMode::NotEditing, true, None)
            }
            KeyCode::Enter => {
                let current_value =
                    &self.get_current_value()[&self.current_field];
                if current_value.is_object() || current_value.is_array() {
                    self.open_current_section();
                    (
                        EditMode::NotEditing,
                        true,
                        Some(SettingsAction::OpenSection),
                    )
                } else {
                    self.start_editing();
                    (EditMode::EditingValue, true, None)
                }
            }
            KeyCode::Left => {
                if !self.current_path.is_empty() {
                    self.close_current_section();
                    (
                        EditMode::NotEditing,
                        true,
                        Some(SettingsAction::CloseSection),
                    )
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
            KeyCode::Char('d') | KeyCode::Char('D') => (
                EditMode::NotEditing,
                true,
                Some(SettingsAction::DeleteCurrentKey),
            ),
            KeyCode::Char('c') | KeyCode::Char('C') => (
                EditMode::NotEditing,
                true,
                Some(SettingsAction::ClearCurrentKey),
            ),
            _ => (EditMode::NotEditing, false, None),
        }
    }

    fn handle_editing_value(
        &mut self,
        key: KeyCode,
    ) -> (EditMode, bool, Option<SettingsAction>) {
        match key {
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
        }
    }

    fn handle_adding_new_key(
        &mut self,
        key: KeyCode,
    ) -> (EditMode, bool, Option<SettingsAction>) {
        match key {
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
        }
    }

    fn handle_adding_new_value(
        &mut self,
        key: KeyCode,
    ) -> (EditMode, bool, Option<SettingsAction>) {
        match key {
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
        }
    }

    pub fn get_display_value(&self, key: &str, value: &JsonValue) -> String {
        if key.starts_with("__section.") {
            let section_type = key.split('.').nth(1).unwrap_or("Unknown");
            let section_name = value["name"].as_str().unwrap_or("Unnamed");
            format!(
                "{}: {}",
                section_type.to_string().capitalize(),
                section_name
            )
        } else {
            match value {
                JsonValue::Object(obj)
                    if obj.get("was_encrypted")
                        == Some(&JsonValue::Bool(true)) =>
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
                JsonValue::Object(_) => "{...}".to_string(),
                JsonValue::Array(_) => "[...]".to_string(),
                JsonValue::String(s) => s.clone(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Null => "null".to_string(),
            }
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

    pub fn get_current_field(&self) -> &str {
        &self.current_field
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

    fn move_selection(&mut self, delta: i32) {
        let current_obj = self.get_current_value().as_object().unwrap();
        let keys: Vec<_> = current_obj.keys().collect();
        let current_index = keys
            .iter()
            .position(|&k| k == &self.current_field)
            .unwrap_or(0);
        let new_index = (current_index as i32 + delta)
            .rem_euclid(keys.len() as i32) as usize;
        self.current_field = keys[new_index].to_string();
    }

    fn start_editing(&mut self) {
        if let Some(value) =
            self.get_current_value()[&self.current_field].as_str()
        {
            self.edit_buffer = value.to_string();
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

    pub fn get_current_index(&self) -> usize {
        self.get_current_value()
            .as_object()
            .unwrap()
            .keys()
            .position(|k| k == &self.current_field)
            .unwrap_or(0)
    }

    pub fn clear(&mut self) {
        self.settings = JsonValue::Object(serde_json::Map::new());
        self.current_path.clear();
        self.current_field = String::new();
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    fn open_current_section(&mut self) {
        self.current_path.push(self.current_field.clone());
        let new_value = if self.current_field.starts_with("__section.") {
            &self.get_current_value()["settings"]
        } else {
            &self.get_current_value()[&self.current_field]
        };
        self.current_field = new_value
            .as_object()
            .and_then(|obj| obj.keys().next().cloned())
            .unwrap_or_default();
    }

    fn close_current_section(&mut self) {
        if let Some(parent_key) = self.current_path.pop() {
            self.current_field = parent_key;
        }
    }

    pub fn get_current_value(&self) -> &JsonValue {
        let mut current = &self.settings;
        for key in &self.current_path {
            current = if key.starts_with("__section.") {
                &current[key]["settings"]
            } else {
                &current[key]
            }
        }
        current
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
