use std::collections::HashSet;
use serde_json::{Map, Value as JsonValue};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsAction {
    ToggleSecureVisibility,
    DeleteCurrentKey,
    ClearCurrentKey,
    SaveEdit,
    SaveNewValue,
    CloseSection,
    ToggleSection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsEditor {
    settings: JsonValue,
    pub current_path: Vec<String>,
    expanded_sections: HashSet<String>,
    pub current_field: String,
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
            expanded_sections: HashSet::new(),
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
                let current_value = self.get_current_value();
                eprintln!("Enter, current_field: {}, current_value: {:?}", self.current_field, current_value);
                
                let field_value = if self.current_field.starts_with("__section.") {
                    Some(current_value)
                } else {
                    current_value.get(&self.current_field)
                };
                
                eprintln!("field_value: {:?}", field_value);

                if let Some(value) = field_value {
                    if value.is_object() || value.is_array() {
                        self.toggle_section_expansion();
                        (EditMode::NotEditing, true, Some(SettingsAction::ToggleSection))
                    } else {
                        self.start_editing();
                        (EditMode::EditingValue, true, None)
                    }
                } else {
                    (EditMode::NotEditing, false, None)
                }
            }
            KeyCode::Left => {
                if !self.current_path.is_empty() {
                    self.close_current_section();
                    (EditMode::NotEditing, true, Some(SettingsAction::CloseSection))
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

    pub fn get_display_value(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::Object(obj) if obj.contains_key("__encryption_key") => {
                let display = if self.show_secure {
                    match obj.get("__content") {
                        Some(JsonValue::String(s)) => s.clone(),
                        _ => "Invalid Value".to_string(),
                    }
                } else {
                    "*****".to_string()
                };
                format!("{} (Encrypted)", display)
            }
            JsonValue::Object(obj) => {
                if obj.is_empty() {
                    "{}".to_string()
                } else if let Some(name) = obj.get("name") {
                    match name {
                        JsonValue::String(s) => format!("{} {{...}}", s),
                        _ => format!("{} {{...}}", name),
                    }
                } else {
                    "{...}".to_string()
                }
            }
            JsonValue::Array(arr) if arr.is_empty() => "[]".to_string(),
            JsonValue::Array(arr) => format!("[{} items]", arr.len()),
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "null".to_string(),
        }
    }

    fn flatten_settings<'a>(
        &'a self,
        prefix: &str,
        value: &'a JsonValue,
        depth: usize,
        result: &mut Vec<(String, &'a JsonValue, usize)>,
    ) {
        match value {
            JsonValue::Object(obj) => {
                let current_path = if prefix.is_empty() {
                    String::new()
                } else {
                    format!("{prefix}.")
                };
                
                for (key, val) in obj {
                    let full_key = format!("{current_path}{key}");
                    result.push((full_key.clone(), val, depth));
                    
                    if self.is_section_expanded(&full_key) && (val.is_object() || val.is_array()) {
                        self.flatten_settings(&full_key, val, depth + 1, result);
                    }
                }
            }
            JsonValue::Array(arr) => {
                for (index, val) in arr.iter().enumerate() {
                    let full_key = format!("{prefix}[{index}]");
                    result.push((full_key.clone(), val, depth));
                    
                    if self.is_section_expanded(&full_key) && (val.is_object() || val.is_array()) {
                        self.flatten_settings(&full_key, val, depth + 1, result);
                    }
                }
            }
            _ => {}
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

    pub fn toggle_section_expansion(&mut self) {
        let full_path = if self.current_path.is_empty() {
            self.current_field.clone()
        } else {
            format!("{}.{}", self.current_path.join("."), self.current_field)
        };

        if self.expanded_sections.contains(&full_path) {
            self.expanded_sections.remove(&full_path);
            self.expanded_sections.retain(|path| !path.starts_with(&format!("{}.", full_path)));
        } else {
            self.expanded_sections.insert(full_path);
        }
    }

    pub fn is_section_expanded(&self, path: &str) -> bool {
        self.expanded_sections.contains(path)
    }

    pub fn get_flattened_settings(&self) -> Vec<(String, &JsonValue, usize)> {
        let mut flattened = Vec::new();
        self.flatten_settings("", &self.settings, 0, &mut flattened);
        flattened
    }

    pub fn move_selection(&mut self, delta: i32) {
        let flattened = self.get_flattened_settings();
        let current_index = flattened
            .iter()
            .position(|(key, _, _)| key == &self.current_field)
            .unwrap_or(0);
        let new_index = (current_index as i32 + delta)
            .rem_euclid(flattened.len() as i32) as usize;
        self.current_field = flattened[new_index].0.clone();
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

    pub fn clear(&mut self) {
        self.settings = JsonValue::Object(serde_json::Map::new());
        self.current_path.clear();
        self.current_field = String::new();
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    fn close_current_section(&mut self) {
        if let Some(parent_key) = self.current_path.pop() {
            self.current_field = parent_key;
        }
    }

    pub fn get_current_value(&self) -> &JsonValue {
        let mut current = &self.settings;
        for key in &self.current_path {
            if key.starts_with("__section.") {
                if let Some(section) = current.get(key) {
                    if let Some(settings) = section.get("settings") {
                        current = settings;
                    } else {
                        current = section;
                    }
                }
            } else {
                current = &current[key];
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
