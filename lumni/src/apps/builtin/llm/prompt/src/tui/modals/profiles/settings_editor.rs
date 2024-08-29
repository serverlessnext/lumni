use crossterm::event::KeyCode;
use serde_json::{json, Map, Value as JsonValue};

use super::profile_edit_modal::EditMode;
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
    show_secure: bool,
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
        }
    }

    pub fn clear(&mut self) {
        self.settings = JsonValue::Object(serde_json::Map::new());
        self.current_field = 0;
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
        self.show_secure = false;
    }

    pub fn get_settings(&self) -> &JsonValue {
        &self.settings
    }

    pub fn move_selection_up(&mut self) {
        if self.current_field > 0 {
            self.current_field -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        let settings_len = self.settings.as_object().map_or(0, |obj| obj.len());
        if settings_len > 0 && self.current_field < settings_len - 1 {
            self.current_field += 1;
        }
    }

    pub fn start_editing(&mut self) -> Option<String> {
        let current_key = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field)
            .unwrap();
        if !current_key.starts_with("__") {
            let value = &self.settings[current_key];
            self.edit_buffer = match value {
                JsonValue::Object(obj)
                    if obj.get("was_encrypted")
                        == Some(&JsonValue::Bool(true)) =>
                {
                    match obj.get("value") {
                        Some(JsonValue::Number(n)) => n.to_string(),
                        Some(JsonValue::String(s)) => s.clone(),
                        _ => "".to_string(),
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
    }

    pub fn start_adding_new_value(&mut self, is_secure: bool) {
        self.new_key_buffer.clear();
        self.edit_buffer.clear();
        self.is_new_value_secure = is_secure;
    }

    pub fn confirm_new_key(&mut self) -> bool {
        !self.new_key_buffer.is_empty()
    }

    pub async fn save_edit(
        &mut self,
        profile: &UserProfile,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        let current_key = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field)
            .unwrap()
            .to_string();

        let current_value = &self.settings[&current_key];
        let is_encrypted = if let Some(obj) = current_value.as_object() {
            obj.contains_key("was_encrypted")
                && obj["was_encrypted"].as_bool().unwrap_or(false)
        } else {
            false
        };

        let new_value = if is_encrypted {
            json!({
                "content": self.edit_buffer,
                "encryption_key": "",   // signal that the value must be encrypted
                "type_info": "string",
            })
        } else {
            serde_json::Value::String(self.edit_buffer.clone())
        };

        let mut update_settings = JsonValue::Object(serde_json::Map::new());
        update_settings[&current_key] = new_value;
        db_handler.update(profile, &update_settings).await?;

        // Reload settings to reflect the changes
        self.load_settings(profile, db_handler).await?;

        Ok(())
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

    pub async fn save_new_value(
        &mut self,
        profile: &UserProfile,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        let new_key = self.new_key_buffer.clone();
        let new_value = if self.is_new_value_secure {
            json!({
                "content": self.edit_buffer.clone(),
                "encryption_key": "",   // signal that value must be encrypted, encryption key will be set by the handler
                "type_info": "string",
            })
        } else {
            serde_json::Value::String(self.edit_buffer.clone())
        };

        let mut update_settings = JsonValue::Object(serde_json::Map::new());
        update_settings[&new_key] = new_value.clone();
        db_handler.update(profile, &update_settings).await?;

        // Update the local settings for feedback to the user
        if let Some(obj) = self.settings.as_object_mut() {
            if self.is_new_value_secure {
                obj.insert(
                    new_key.clone(),
                    json!({
                        "content": self.edit_buffer.clone(),
                        "was_encrypted": true
                    }),
                );
            } else {
                obj.insert(new_key.clone(), new_value);
            }
        }

        // Set the current field to the newly added key
        self.current_field = self.find_key_index(&new_key);

        // Reset buffers and flags
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;

        Ok(())
    }

    fn find_key_index(&self, key: &str) -> usize {
        self.settings
            .as_object()
            .map(|obj| obj.keys().position(|k| k == key).unwrap_or(0))
            .unwrap_or(0)
    }

    pub fn cancel_edit(&mut self) {
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    pub async fn delete_current_key(
        &mut self,
        profile: &UserProfile,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field)
        {
            let current_key = current_key.to_string();
            if !current_key.starts_with("__") {
                let mut settings = Map::new();
                settings.insert(current_key, JsonValue::Null); // Null indicates deletion
                db_handler
                    .update(profile, &JsonValue::Object(settings))
                    .await?;
                self.load_settings(profile, db_handler).await?;
            }
        }
        Ok(())
    }

    pub async fn clear_current_key(
        &mut self,
        profile: &UserProfile,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field)
        {
            let current_key = current_key.to_string();
            if !current_key.starts_with("__") {
                self.settings[&current_key] = JsonValue::String("".to_string());
                db_handler.update(profile, &self.settings).await?;
            }
        }
        Ok(())
    }

    pub async fn toggle_secure_visibility(
        &mut self,
        profile: &UserProfile,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        // Store the current key before toggling
        let current_key = self.get_current_key().map(String::from);

        self.show_secure = !self.show_secure;
        self.load_settings(profile, db_handler).await?;

        // Restore the selection after reloading settings
        if let Some(key) = current_key {
            self.current_field = self.find_key_index(&key);
        }

        Ok(())
    }

    fn get_current_key(&self) -> Option<&str> {
        self.settings
            .as_object()
            .and_then(|obj| obj.keys().nth(self.current_field))
            .map(String::as_str)
    }

    pub async fn load_settings(
        &mut self,
        profile: &UserProfile,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        let mask_mode = if self.show_secure {
            MaskMode::Unmask
        } else {
            MaskMode::Mask
        };
        self.settings =
            db_handler.get_profile_settings(profile, mask_mode).await?;
        eprintln!("Settings: {:?}", self.settings);
        self.current_field = 0;
        Ok(())
    }

    pub fn handle_key_event(
        &mut self,
        key_code: KeyCode,
        current_mode: EditMode,
    ) -> (EditMode, bool, Option<SettingsAction>) {
        match current_mode {
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
            EditMode::RenamingProfile => {
                (EditMode::RenamingProfile, false, None)
            }
        }
    }

    // Getter methods for UI rendering
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
}
