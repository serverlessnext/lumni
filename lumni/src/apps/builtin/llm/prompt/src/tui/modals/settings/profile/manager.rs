use serde_json::{json, Map, Value as JsonValue};
use tokio::sync::mpsc;

use super::creator::{ProfileCreator, ProfileCreatorAction};
use super::list::ProfileList;
use super::*;

pub struct ProfileManager {
    pub list: ProfileList,
    pub settings_editor: SettingsEditor,
    pub creator: Option<ProfileCreator>,
    rename_buffer: Option<String>,
    db_handler: UserProfileDbHandler,
}

impl ProfileManager {
    pub async fn new(
        mut db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let profiles = db_handler.list_profiles().await?;
        let default_profile = db_handler.get_default_profile().await?;
        let list = ProfileList::new(profiles, default_profile);

        let settings = if let Some(profile) = list.get_selected_profile() {
            db_handler
                .get_profile_settings(profile, MaskMode::Mask)
                .await?
        } else {
            JsonValue::Object(serde_json::Map::new())
        };
        let settings_editor = SettingsEditor::new(settings);

        Ok(Self {
            list,
            settings_editor,
            creator: None,
            rename_buffer: None,
            db_handler,
        })
    }

    pub async fn refresh_profile_list(
        &mut self,
    ) -> Result<WindowEvent, ApplicationError> {
        if let Some(creator) = &mut self.creator {
            // Profile is being created in the background
            if let Some(ref mut rx) = creator.background_task {
                match rx.try_recv() {
                    Ok(BackgroundTaskResult::ProfileCreated(result)) => {
                        creator.background_task = None;
                        creator.task_start_time = None;
                        match result {
                            Ok(new_profile) => {
                                self.list.add_profile(new_profile);
                                self.creator = None;
                            }
                            Err(e) => {
                                log::error!("Failed to create profile: {}", e);
                            }
                        }
                        return Ok(WindowEvent::Modal(ModalAction::Refresh));
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {
                        return Ok(WindowEvent::Modal(ModalAction::Refresh));
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        self.creator = None;
                        return Ok(WindowEvent::Modal(ModalAction::Refresh));
                    }
                }
            }
        }
        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowEvent, ApplicationError> {
        match *tab_focus {
            TabFocus::List => {
                self.handle_list_input(key_event, tab_focus).await
            }
            TabFocus::Settings => {
                self.handle_settings_input(key_event, tab_focus).await
            }
            TabFocus::Creation => {
                self.handle_creation_input(key_event, tab_focus).await
            }
        }
    }

    async fn handle_settings_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowEvent, ApplicationError> {
        let (new_mode, handled, action) =
            self.settings_editor.handle_key_event(key_event.code);

        if handled {
            self.settings_editor.edit_mode = new_mode;

            if let Some(action) = action {
                // Get the profile outside the mutable borrow
                let profile = self.list.get_selected_profile().cloned();

                if let Some(profile) = profile {
                    match action {
                        SettingsAction::ToggleSecureVisibility => {
                            self.toggle_secure_visibility(&profile).await?;
                        }
                        SettingsAction::DeleteCurrentKey => {
                            self.delete_current_key(&profile).await?;
                        }
                        SettingsAction::ClearCurrentKey => {
                            self.clear_current_key(&profile).await?;
                        }
                        SettingsAction::SaveEdit => {
                            self.save_edit(&profile).await?;
                        }
                        SettingsAction::SaveNewValue => {
                            self.save_new_value(&profile).await?;
                        }
                    }
                }
            }

            return Ok(WindowEvent::Modal(ModalAction::Refresh));
        }

        if self.settings_editor.edit_mode == EditMode::NotEditing
            && (key_event.code == KeyCode::Left
                || key_event.code == KeyCode::Char('q')
                || key_event.code == KeyCode::Esc
                || key_event.code == KeyCode::Tab)
        {
            *tab_focus = TabFocus::List;
            return Ok(WindowEvent::Modal(ModalAction::Refresh));
        }

        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
    }

    async fn toggle_secure_visibility(
        &mut self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        let new_mask_mode = if self.settings_editor.show_secure {
            MaskMode::Mask
        } else {
            MaskMode::Unmask
        };
        let settings = self
            .db_handler
            .get_profile_settings(profile, new_mask_mode)
            .await?;
        self.settings_editor.load_settings(settings);
        self.settings_editor.show_secure = !self.settings_editor.show_secure;
        Ok(())
    }

    async fn delete_current_key(
        &mut self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            if !current_key.starts_with("__") {
                let mut settings = Map::new();
                settings.insert(current_key.to_string(), JsonValue::Null); // Null indicates deletion
                self.db_handler
                    .update(profile, &JsonValue::Object(settings))
                    .await?;
                self.load_profile_settings(profile).await?;
            }
        }
        Ok(())
    }

    async fn clear_current_key(
        &mut self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            if !current_key.starts_with("__") {
                let mut settings = self.settings_editor.get_settings().clone();
                settings[current_key] = JsonValue::String("".to_string());
                self.db_handler.update(profile, &settings).await?;
                self.load_profile_settings(profile).await?;
            }
        }
        Ok(())
    }

    async fn save_edit(
        &mut self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            let current_value =
                &self.settings_editor.get_settings()[current_key];
            let is_encrypted = if let Some(obj) = current_value.as_object() {
                obj.contains_key("was_encrypted")
                    && obj["was_encrypted"].as_bool().unwrap_or(false)
            } else {
                false
            };

            let new_value = if is_encrypted {
                json!({
                    "content": self.settings_editor.get_edit_buffer(),
                    "encryption_key": "",   // signal that the value must be encrypted
                    "type_info": "string",
                })
            } else {
                serde_json::Value::String(
                    self.settings_editor.get_edit_buffer().to_string(),
                )
            };

            let mut update_settings = JsonValue::Object(serde_json::Map::new());
            update_settings[current_key] = new_value;
            self.db_handler.update(profile, &update_settings).await?;

            self.load_profile_settings(profile).await?;
        }
        Ok(())
    }

    async fn save_new_value(
        &mut self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        let new_key = self.settings_editor.get_new_key_buffer().to_string();
        let new_value = if self.settings_editor.is_new_value_secure() {
            json!({
                "content": self.settings_editor.get_edit_buffer(),
                "encryption_key": "",   // signal that value must be encrypted, encryption key will be set by the handler
                "type_info": "string",
            })
        } else {
            serde_json::Value::String(
                self.settings_editor.get_edit_buffer().to_string(),
            )
        };

        let mut update_settings = JsonValue::Object(serde_json::Map::new());
        update_settings[&new_key] = new_value.clone();
        self.db_handler.update(profile, &update_settings).await?;

        // Update the local settings for feedback to the user
        if let Some(obj) =
            self.settings_editor.get_settings_mut().as_object_mut()
        {
            obj.insert(new_key.clone(), new_value);
        }

        // Reload settings to reflect changes
        self.load_profile_settings(profile).await?;

        Ok(())
    }

    async fn load_profile_settings(
        &mut self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        let settings = self
            .db_handler
            .get_profile_settings(profile, MaskMode::Mask)
            .await?;
        self.settings_editor.load_settings(settings);
        Ok(())
    }
    async fn handle_list_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowEvent, ApplicationError> {
        match key_event.code {
            KeyCode::Up => {
                if self.rename_buffer.is_some() {
                    self.confirm_rename_profile().await?;
                }
                if self.list.move_selection_up() {
                    self.load_selected_profile_settings().await?;
                }
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Down => {
                if self.rename_buffer.is_some() {
                    self.confirm_rename_profile().await?;
                }
                if self.list.move_selection_down() {
                    self.load_selected_profile_settings().await?;
                }
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Enter => {
                if self.list.is_new_profile_selected() {
                    self.start_profile_creation().await?;
                    *tab_focus = TabFocus::Creation;
                } else if self.rename_buffer.is_some() {
                    self.confirm_rename_profile().await?;
                } else {
                    *tab_focus = TabFocus::Settings;
                }
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.start_profile_renaming();
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Char(c) if self.rename_buffer.is_some() => {
                if let Some(buffer) = &mut self.rename_buffer {
                    buffer.push(c);
                }
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Backspace if self.rename_buffer.is_some() => {
                if let Some(buffer) = &mut self.rename_buffer {
                    buffer.pop();
                }
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Esc if self.rename_buffer.is_some() => {
                self.cancel_rename_profile();
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Char(' ') => {
                self.set_default_profile().await?;
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Char('D') => {
                self.delete_selected_profile().await?;
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            _ => Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent)),
        }
    }

    pub async fn handle_creation_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowEvent, ApplicationError> {
        if let Some(creator) = &mut self.creator {
            match creator.handle_input(key_event).await? {
                ProfileCreatorAction::Refresh => {
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProfileCreatorAction::WaitForKeyEvent => {
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                ProfileCreatorAction::Cancel => {
                    self.creator = None;
                    *tab_focus = TabFocus::List;
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProfileCreatorAction::CreateProfile => {
                    let new_profile =
                        creator.create_profile(&mut self.db_handler).await?;
                    self.list.add_profile(new_profile);
                    self.creator = None;
                    *tab_focus = TabFocus::List;
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProfileCreatorAction::SwitchToProviderCreation => {
                    // This action is handled within the ProfileCreator
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProfileCreatorAction::FinishProviderCreation(new_config) => {
                    // Update the ProviderManager with the new config
                    // This might require passing a reference to ProviderManager to ProfileManager
                    // For now, we'll just refresh
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
            }
        } else {
            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
        }
    }

    async fn load_selected_profile_settings(
        &mut self,
    ) -> Result<(), ApplicationError> {
        if let Some(profile) = self.list.get_selected_profile() {
            let settings = self
                .db_handler
                .get_profile_settings(profile, MaskMode::Mask)
                .await?;
            self.settings_editor.load_settings(settings);
        } else {
            // Clear settings when "Create new Profile" is selected
            self.settings_editor.clear();
        }
        Ok(())
    }

    async fn start_profile_creation(&mut self) -> Result<(), ApplicationError> {
        self.creator =
            Some(ProfileCreator::new(self.db_handler.clone()).await?);
        Ok(())
    }

    fn start_profile_renaming(&mut self) {
        if let Some(profile) = self.list.get_selected_profile() {
            self.rename_buffer = Some(profile.name.clone());
        }
    }

    async fn confirm_rename_profile(&mut self) -> Result<(), ApplicationError> {
        if let (Some(new_name), Some(profile)) =
            (&self.rename_buffer, self.list.get_selected_profile())
        {
            if !new_name.is_empty() {
                self.db_handler.rename_profile(profile, new_name).await?;
                self.list.rename_selected_profile(new_name.clone());
            }
        }
        self.rename_buffer = None;
        Ok(())
    }

    fn cancel_rename_profile(&mut self) {
        self.rename_buffer = None;
    }

    async fn set_default_profile(&mut self) -> Result<(), ApplicationError> {
        if let Some(profile) = self.list.get_selected_profile().cloned() {
            self.db_handler.set_default_profile(&profile).await?;
            self.list.mark_as_default(&profile);
        }
        Ok(())
    }

    async fn delete_selected_profile(
        &mut self,
    ) -> Result<(), ApplicationError> {
        self.list.delete_profile(&mut self.db_handler).await?;
        self.load_selected_profile_settings().await?;
        self.rename_buffer = None;
        Ok(())
    }

    pub fn get_rename_buffer(&self) -> Option<&String> {
        self.rename_buffer.as_ref()
    }
}
