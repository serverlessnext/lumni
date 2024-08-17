use super::*;

pub struct SettingsEditor {
    settings: Value,
    current_field: usize,
    edit_buffer: String,
    new_key_buffer: String,
    is_new_value_secure: bool,
    show_secure: bool,
}

impl SettingsEditor {
    pub fn new(settings: Value) -> Self {
        Self {
            settings,
            current_field: 0,
            edit_buffer: String::new(),
            new_key_buffer: String::new(),
            is_new_value_secure: false,
            show_secure: false,
        }
    }

    pub fn get_settings(&self) -> &Value {
        &self.settings
    }

    pub fn move_selection_up(&mut self) {
        if self.current_field > 0 {
            self.current_field -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.current_field < self.settings.as_object().unwrap().len() - 1 {
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
            self.edit_buffer = self.settings[current_key]
                .as_str()
                .unwrap_or("")
                .to_string();
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
        profile: &str,
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
        self.settings[&current_key] = Value::String(self.edit_buffer.clone());
        db_handler.create_or_update(profile, &self.settings).await
    }

    pub async fn save_new_value(
        &mut self,
        profile: &str,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        if self.is_new_value_secure {
            self.settings[&self.new_key_buffer] = json!({
                "value": self.edit_buffer,
                "was_encrypted": true
            });
        } else {
            self.settings[&self.new_key_buffer] =
                Value::String(self.edit_buffer.clone());
        }
        db_handler.create_or_update(profile, &self.settings).await
    }

    pub fn cancel_edit(&mut self) {
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    pub async fn delete_current_key(
        &mut self,
        profile: &str,
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
                settings.insert(current_key, Value::Null); // Null indicates deletion
                db_handler
                    .create_or_update(profile, &Value::Object(settings))
                    .await?;
                self.load_settings(profile, db_handler).await?;
            }
        }
        Ok(())
    }

    pub async fn clear_current_key(
        &mut self,
        profile: &str,
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
                self.settings[&current_key] = Value::String("".to_string());
                db_handler.create_or_update(profile, &self.settings).await?;
            }
        }
        Ok(())
    }

    pub fn toggle_secure_visibility(&mut self) {
        self.show_secure = !self.show_secure;
    }

    pub async fn load_settings(
        &mut self,
        profile: &str,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        let mask_mode = if self.show_secure {
            MaskMode::Unmask
        } else {
            MaskMode::Mask
        };
        self.settings =
            db_handler.get_profile_settings(profile, mask_mode).await?;
        self.current_field = 0;
        Ok(())
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

    pub fn is_show_secure(&self) -> bool {
        self.show_secure
    }

    pub fn set_edit_buffer(&mut self, value: String) {
        self.edit_buffer = value;
    }

    pub fn set_new_key_buffer(&mut self, value: String) {
        self.new_key_buffer = value;
    }
}
