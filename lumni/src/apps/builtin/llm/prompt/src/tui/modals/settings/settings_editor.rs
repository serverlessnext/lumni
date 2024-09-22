use std::collections::HashSet;

use ratatui::style::Stylize;
use serde_json::Value as JsonValue;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsAction {
    ToggleSecureVisibility,
    DeleteCurrentKey,
    ClearCurrentKey,
    SaveEdit,
    SaveNewValue,
    ToggleSection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsEditor {
    settings: JsonValue,
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
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    pub fn set_current_field(&mut self, field: String) {
        self.current_field = field;
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

    fn get_nested_value(&self, field: &str) -> Option<&JsonValue> {
        let (first_key, remaining_path) = if field.starts_with("__section.") {
            let (section, rest) = field.split_at(field.find('.').unwrap() + 1);
            let (provider, rest) =
                rest.split_at(rest.find('.').unwrap_or(rest.len()));
            (format!("{}{}", section, provider), rest)
        } else {
            let parts: Vec<&str> = field.splitn(2, '.').collect();
            (parts[0].to_string(), parts.get(1).copied().unwrap_or(""))
        };

        let mut current = self.settings.get(&first_key)?;

        for part in remaining_path.split('.').filter(|&p| !p.is_empty()) {
            current = match current {
                JsonValue::Object(obj) => obj.get(part)?,
                _ => return None,
            };
        }

        Some(current)
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
                if let Some(current_value) =
                    self.get_nested_value(&self.current_field)
                {
                    match current_value {
                        JsonValue::Object(obj)
                            if obj.contains_key("__content") =>
                        {
                            let is_encrypted =
                                obj.contains_key("__encryption_key");
                            if is_encrypted && !self.show_secure {
                                // Encrypted value but masked, don't allow editing
                                (EditMode::NotEditing, true, None)
                            } else {
                                // Allow editing for non-encrypted or unmasked encrypted values
                                self.start_editing();
                                (EditMode::EditingValue, true, None)
                            }
                        }
                        JsonValue::Object(obj) => {
                            if obj.is_empty() {
                                // Empty object, don't expand
                                (EditMode::NotEditing, false, None)
                            } else {
                                // Non-empty object, expand it
                                self.toggle_section_expansion();
                                (
                                    EditMode::NotEditing,
                                    true,
                                    Some(SettingsAction::ToggleSection),
                                )
                            }
                        }
                        JsonValue::Array(arr) => {
                            if arr.is_empty() {
                                // Empty array, don't expand
                                (EditMode::NotEditing, false, None)
                            } else {
                                // Non-empty array, expand it
                                self.toggle_section_expansion();
                                (
                                    EditMode::NotEditing,
                                    true,
                                    Some(SettingsAction::ToggleSection),
                                )
                            }
                        }
                        _ => {
                            // Regular value, start editing
                            self.start_editing();
                            (EditMode::EditingValue, true, None)
                        }
                    }
                } else {
                    // Field not found, do nothing
                    (EditMode::NotEditing, false, None)
                }
            }
            KeyCode::Left => (EditMode::NotEditing, false, None),
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

    pub fn get_display_value_span(&self, value: &JsonValue) -> Span<'static> {
        match value {
            JsonValue::Object(obj) if obj.contains_key("__content") => {
                let content = obj.get("__content").unwrap();
                let placeholder =
                    obj.get("__placeholder").and_then(|v| v.as_str());

                if obj.contains_key("__encryption_key") {
                    // Encrypted value
                    if self.show_secure {
                        if let JsonValue::String(s) = content {
                            Span::styled(
                                s.clone(),
                                Style::default().fg(Color::Cyan),
                            )
                        } else {
                            Span::styled(
                                "Invalid Value",
                                Style::default().fg(Color::Red),
                            )
                        }
                    } else {
                        Span::styled(
                            "*****",
                            Style::default().fg(Color::DarkGray),
                        )
                    }
                } else {
                    // Non-encrypted value
                    match content {
                        JsonValue::String(s) if s.is_empty() => {
                            if let Some(placeholder_text) = placeholder {
                                Span::styled(
                                    format!("({})", placeholder_text),
                                    Style::default()
                                        .fg(Color::DarkGray)
                                        .italic(),
                                )
                            } else {
                                Span::styled(
                                    "",
                                    Style::default().fg(Color::DarkGray),
                                )
                            }
                        }
                        JsonValue::String(s) => Span::styled(
                            s.clone(),
                            Style::default().fg(Color::Cyan),
                        ),
                        _ => Span::styled(
                            "Invalid Value",
                            Style::default().fg(Color::Red),
                        ),
                    }
                }
            }
            JsonValue::Object(obj) => {
                if let Some(type_name) =
                    obj.get("__type").and_then(|v| v.as_str())
                {
                    Span::styled(
                        format!("{} {{...}}", type_name),
                        Style::default().fg(Color::Cyan),
                    )
                } else {
                    if obj.is_empty() {
                        Span::styled("{}", Style::default().fg(Color::DarkGray))
                    } else {
                        Span::styled("{...}", Style::default().fg(Color::Cyan))
                    }
                }
            }
            JsonValue::Array(arr) => Span::styled(
                format!("[{} items]", arr.len()),
                Style::default().fg(Color::Cyan),
            ),
            JsonValue::String(s) => {
                Span::styled(s.clone(), Style::default().fg(Color::Cyan))
            }
            JsonValue::Number(n) => {
                Span::styled(n.to_string(), Style::default().fg(Color::Cyan))
            }
            JsonValue::Bool(b) => {
                Span::styled(b.to_string(), Style::default().fg(Color::Cyan))
            }
            JsonValue::Null => {
                Span::styled("null", Style::default().fg(Color::Cyan))
            }
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

                    if self.is_section_expanded(&full_key)
                        && (val.is_object() || val.is_array())
                    {
                        self.flatten_settings(
                            &full_key,
                            val,
                            depth + 1,
                            result,
                        );
                    }
                }
            }
            JsonValue::Array(arr) => {
                for (index, val) in arr.iter().enumerate() {
                    let full_key = format!("{prefix}[{index}]");
                    result.push((full_key.clone(), val, depth));

                    if self.is_section_expanded(&full_key)
                        && (val.is_object() || val.is_array())
                    {
                        self.flatten_settings(
                            &full_key,
                            val,
                            depth + 1,
                            result,
                        );
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
        if self.expanded_sections.contains(&self.current_field) {
            self.expanded_sections.remove(&self.current_field);
            self.expanded_sections.retain(|path| {
                !path.starts_with(&format!("{}.", self.current_field))
            });
        } else {
            self.expanded_sections.insert(self.current_field.clone());
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
            .rem_euclid(flattened.len() as i32)
            as usize;
        self.current_field = flattened[new_index].0.clone();
    }

    fn start_editing(&mut self) {
        if let Some(value) = self.get_nested_value(&self.current_field) {
            self.edit_buffer = match value {
                JsonValue::Object(obj) if obj.contains_key("__content") => {
                    match obj.get("__content") {
                        Some(JsonValue::String(s)) => s.clone(),
                        _ => String::new(),
                    }
                }
                JsonValue::String(s) => s.clone(),
                _ => String::new(),
            };
        } else {
            self.edit_buffer = String::new();
        }
        self.edit_mode = EditMode::EditingValue;
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
        self.current_field = String::new();
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
