use std::any::Any;

use async_trait::async_trait;
use ratatui::prelude::*;
use serde_json::{json, Value as JsonValue};

use super::profile::{
    ProfileCreationStep, ProfileCreator, SubPartCreationState,
};
use super::provider::{ProviderCreationStep, ProviderCreator};
use super::*;

#[async_trait]
pub trait ManagedItem: Clone + Send + Sync + ListItemTrait {
    async fn delete(
        &self,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError>;
    async fn get_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError>;
    async fn update_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        settings: &JsonValue,
    ) -> Result<(), ApplicationError>;
}

#[async_trait]
impl ManagedItem for UserProfile {
    async fn delete(
        &self,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        db_handler.delete_profile(self).await
    }

    async fn get_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        db_handler.unlock_profile_settings(&self).await?;
        let settings = db_handler.get_profile_settings(self, mask_mode).await?;
        Ok(settings)
    }

    async fn update_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        db_handler.update_profile(self, settings).await
    }
}

#[async_trait]
impl ManagedItem for ProviderConfig {
    async fn delete(
        &self,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(id) = self.id {
            db_handler.delete_provider_config(id).await
        } else {
            Ok(())
        }
    }

    async fn get_settings(
        &self,
        _db_handler: &mut UserProfileDbHandler,
        _mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        let mut settings = serde_json::Map::new();

        // Add provider_type (server)
        settings.insert(
            "provider_type".to_string(),
            JsonValue::String(self.provider_type.clone()),
        );

        // Add model_identifier if present
        if let Some(model) = &self.model_identifier {
            settings.insert(
                "model_identifier".to_string(),
                JsonValue::String(model.clone()),
            );
        }

        // Add other additional settings
        for (key, value) in &self.additional_settings {
            settings
                .insert(key.clone(), JsonValue::String(value.value.clone()));
        }

        Ok(JsonValue::Object(settings))
    }

    async fn update_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        let mut updated_config = self.clone();
        if let JsonValue::Object(map) = settings {
            for (key, value) in map {
                if let Some(setting) =
                    updated_config.additional_settings.get_mut(key)
                {
                    setting.value =
                        value.as_str().unwrap_or_default().to_string();
                }
            }
        }
        _ = db_handler.save_provider_config(&updated_config).await?;
        Ok(())
    }
}

#[async_trait]
pub trait Creator<T: ManagedItem>: Send + Sync + 'static {
    async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<T>, ApplicationError>;
    fn render(&mut self, f: &mut Frame, area: Rect);
    async fn create_item(
        &mut self,
    ) -> Result<CreatorAction<T>, ApplicationError>;
    fn poll_background_task(&mut self) -> Option<CreatorAction<T>>;
}

pub enum CreatorAction<T: ManagedItem> {
    Continue, // continue to next step
    Cancel,
    Finish(T),
    LoadAdditionalSettings,
    CreateItem, // spawn a background task to create the item
}

pub struct SettingsManager<T: ManagedItem + LoadableItem + CreatableItem> {
    pub list: SettingsList<T>,
    pub settings_editor: SettingsEditor,
    pub creator: Option<Box<dyn Creator<T>>>,
    pub rename_buffer: Option<String>,
    pub db_handler: UserProfileDbHandler,
    pub tab_focus: TabFocus,
}

impl<T: ManagedItem + LoadableItem + CreatableItem + 'static>
    SettingsManager<T>
{
    pub async fn new(
        mut db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let items = T::load_items(&mut db_handler).await?;
        let default_item = T::load_default_item(&mut db_handler).await?;
        let list = SettingsList::new(items, default_item);

        let settings = if let Some(item) = list.get_selected_item() {
            item.get_settings(&mut db_handler, MaskMode::Mask).await?
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
            tab_focus: TabFocus::List,
        })
    }

    pub async fn refresh_list(
        &mut self,
    ) -> Result<WindowMode, ApplicationError> {
        let items = T::load_items(&mut self.db_handler).await?;
        let default_item = T::load_default_item(&mut self.db_handler).await?;

        // Preserve the current default item if a new one wasn't loaded
        let default_item =
            default_item.or_else(|| self.list.default_item.clone());

        self.list = SettingsList::new(items, default_item);
        self.load_selected_item_settings().await?;
        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowMode, ApplicationError> {
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

    async fn handle_list_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowMode, ApplicationError> {
        match key_event.code {
            KeyCode::Up => {
                if self.rename_buffer.is_some() {
                    self.confirm_rename_item().await?;
                }
                if self.list.move_selection_up() {
                    self.load_selected_item_settings().await?;
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Down => {
                if self.rename_buffer.is_some() {
                    self.confirm_rename_item().await?;
                }
                if self.list.move_selection_down() {
                    self.load_selected_item_settings().await?;
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Enter => {
                if self.rename_buffer.is_some() {
                    // ensure this is matched before any other action
                    self.confirm_rename_item().await?;
                } else if self.list.is_new_item_selected() {
                    self.start_item_creation().await?;
                    *tab_focus = TabFocus::Creation;
                } else {
                    *tab_focus = TabFocus::Settings;
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Char(' ') => {
                let selected_item = self.list.get_selected_item().cloned();
                if let Some(item) = selected_item {
                    if let Some(profile) =
                        (&item as &dyn Any).downcast_ref::<UserProfile>()
                    {
                        self.list.mark_as_default(&item);
                        self.db_handler.set_default_profile(profile).await?;
                        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                    } else {
                        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                    }
                } else {
                    Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                }
            }
            KeyCode::Backspace if self.get_rename_buffer().is_some() => {
                if let Some(buffer) = &mut self.rename_buffer {
                    buffer.pop();
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Char(c) if self.rename_buffer.is_some() => {
                // ensure this is matched before any other char
                if let Some(buffer) = &mut self.rename_buffer {
                    buffer.push(c);
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.start_item_renaming();
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Backspace if self.rename_buffer.is_some() => {
                if let Some(buffer) = &mut self.rename_buffer {
                    buffer.pop();
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Esc if self.rename_buffer.is_some() => {
                self.cancel_rename_item();
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Char('D') => {
                self.delete_selected_item().await?;
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            _ => Ok(WindowMode::Modal(ModalEvent::UpdateUI)),
        }
    }

    async fn handle_settings_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowMode, ApplicationError> {
        // Handle key event in settings editor
        let (new_mode, handled, action) =
            self.settings_editor.handle_key_event(key_event.code);

        if handled {
            self.settings_editor.edit_mode = new_mode;

            if let Some(action) = action {
                // Get the profile outside the mutable borrow
                let item = self.list.get_selected_item().cloned();

                if let Some(ref item) = item {
                    match action {
                        SettingsAction::ToggleSecureVisibility => {
                            self.toggle_secure_visibility(item).await?
                        }
                        SettingsAction::DeleteCurrentKey => {
                            self.delete_current_key(item).await?
                        }
                        SettingsAction::ClearCurrentKey => {
                            self.clear_current_key(item).await?
                        }
                        SettingsAction::SaveEdit => {
                            self.save_edit(item).await?
                        }
                        SettingsAction::SaveNewValue => {
                            self.save_new_value(item).await?
                        }
                    }
                }
            }

            return Ok(WindowMode::Modal(ModalEvent::UpdateUI));
        }

        if self.settings_editor.edit_mode == EditMode::NotEditing
            && (key_event.code == KeyCode::Left
                || key_event.code == KeyCode::Char('q')
                || key_event.code == KeyCode::Esc
                || key_event.code == KeyCode::Tab)
        {
            *tab_focus = TabFocus::List;
            return Ok(WindowMode::Modal(ModalEvent::UpdateUI));
        }

        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
    }

    async fn handle_creation_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowMode, ApplicationError> {
        if let Some(creator) = &mut self.creator {
            let action = creator.handle_input(key_event).await?;
            match action {
                CreatorAction::Continue => {
                    Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                }
                CreatorAction::Cancel => {
                    self.creator = None;
                    *tab_focus = TabFocus::List;
                    Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                }
                CreatorAction::Finish(new_item) => {
                    self.list.add_item(new_item);
                    self.creator = None;
                    *tab_focus = TabFocus::List;
                    Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                }
                CreatorAction::CreateItem => {
                    let result = creator.create_item().await?;
                    match result {
                        CreatorAction::Finish(new_item) => {
                            self.list.add_item(new_item);
                            self.creator = None;
                            *tab_focus = TabFocus::List;
                            Ok(WindowMode::Modal(
                                ModalEvent::PollBackGroundTask,
                            ))
                        }
                        _ => Ok(WindowMode::Modal(
                            ModalEvent::PollBackGroundTask,
                        )),
                    }
                }
                CreatorAction::LoadAdditionalSettings => {
                    // Handle loading additional settings if needed
                    Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                }
            }
        } else {
            Ok(WindowMode::Modal(ModalEvent::UpdateUI))
        }
    }

    async fn load_selected_item_settings(
        &mut self,
    ) -> Result<(), ApplicationError> {
        if let Some(item) = self.list.get_selected_item() {
            let mask_mode = if self.settings_editor.show_secure {
                MaskMode::Unmask
            } else {
                MaskMode::Mask
            };
            let settings =
                item.get_settings(&mut self.db_handler, mask_mode).await?;
            self.settings_editor.load_settings(settings);
        } else {
            self.settings_editor.clear();
        }
        Ok(())
    }

    async fn start_item_creation(&mut self) -> Result<(), ApplicationError> {
        let creator = T::create_creator(self.db_handler.clone()).await?;
        self.creator = Some(Box::new(creator));
        Ok(())
    }

    pub fn start_item_renaming(&mut self) {
        if let Some(item) = self.list.get_selected_item() {
            self.rename_buffer = Some(item.name().to_string());
        }
    }

    async fn confirm_rename_item(&mut self) -> Result<(), ApplicationError> {
        if let (Some(new_name), Some(item)) =
            (&self.rename_buffer, self.list.get_selected_item())
        {
            if !new_name.is_empty() {
                // Use type-specific renaming method
                self.rename_item(item, new_name).await?;
                self.list.rename_selected_item(new_name.clone());
            }
        }
        self.rename_buffer = None;
        Ok(())
    }

    async fn rename_item(
        &self,
        item: &T,
        new_name: &str,
    ) -> Result<(), ApplicationError> {
        if let Some(profile) = (item as &dyn Any).downcast_ref::<UserProfile>()
        {
            self.db_handler.rename_profile(profile, new_name).await?;
        } else if let Some(provider) =
            (item as &dyn Any).downcast_ref::<ProviderConfig>()
        {
            self.db_handler
                .rename_provider_config(provider, new_name)
                .await?;
        }
        Ok(())
    }

    pub fn cancel_rename_item(&mut self) {
        self.rename_buffer = None;
    }

    async fn delete_selected_item(&mut self) -> Result<(), ApplicationError> {
        if let Some(item) = self.list.get_selected_item().cloned() {
            item.delete(&mut self.db_handler).await?;
            self.list.remove_selected_item();
            self.load_selected_item_settings().await?;
        }
        self.rename_buffer = None;
        Ok(())
    }

    async fn toggle_secure_visibility(
        &mut self,
        item: &T,
    ) -> Result<(), ApplicationError> {
        self.settings_editor.show_secure = !self.settings_editor.show_secure;
        let mask_mode = if self.settings_editor.show_secure {
            MaskMode::Unmask
        } else {
            MaskMode::Mask
        };
        let settings =
            item.get_settings(&mut self.db_handler, mask_mode).await?;
        self.settings_editor.load_settings(settings);
        Ok(())
    }

    async fn delete_current_key(
        &mut self,
        item: &T,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            if !current_key.starts_with("__") {
                let mut settings = self.settings_editor.get_settings().clone();
                settings[current_key] = JsonValue::Null; // Null indicates deletion
                item.update_settings(&mut self.db_handler, &settings)
                    .await?;
                self.load_selected_item_settings().await?;
            }
        }
        Ok(())
    }

    async fn clear_current_key(
        &mut self,
        item: &T,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            if !current_key.starts_with("__") {
                let mut settings = self.settings_editor.get_settings().clone();
                settings[current_key] = JsonValue::String("".to_string());
                item.update_settings(&mut self.db_handler, &settings)
                    .await?;
                self.load_selected_item_settings().await?;
            }
        }
        Ok(())
    }

    async fn save_edit(&mut self, item: &T) -> Result<(), ApplicationError> {
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

            let mut settings = self.settings_editor.get_settings().clone();
            settings[current_key] = new_value;
            item.update_settings(&mut self.db_handler, &settings)
                .await?;

            self.load_selected_item_settings().await?;
        }
        Ok(())
    }

    async fn save_new_value(
        &mut self,
        item: &T,
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

        let mut settings = self.settings_editor.get_settings().clone();
        settings[&new_key] = new_value.clone();
        item.update_settings(&mut self.db_handler, &settings)
            .await?;

        self.load_selected_item_settings().await?;

        Ok(())
    }

    pub fn get_rename_buffer(&self) -> Option<&String> {
        self.rename_buffer.as_ref()
    }
}

#[async_trait]
pub trait LoadableItem: ManagedItem {
    async fn load_items(
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<Vec<Self>, ApplicationError>;
    async fn load_default_item(
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<Option<Self>, ApplicationError>;
}

#[async_trait]
impl LoadableItem for UserProfile {
    async fn load_items(
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<Vec<Self>, ApplicationError> {
        db_handler.list_profiles().await
    }
    async fn load_default_item(
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<Option<Self>, ApplicationError> {
        db_handler.get_default_profile().await
    }
}

#[async_trait]
impl LoadableItem for ProviderConfig {
    async fn load_items(
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<Vec<Self>, ApplicationError> {
        db_handler.load_provider_configs().await
    }
    async fn load_default_item(
        _db_handler: &mut UserProfileDbHandler,
    ) -> Result<Option<Self>, ApplicationError> {
        Ok(None) // ProviderConfig doesn't have a default item
    }
}

#[async_trait]
pub trait CreatableItem: ManagedItem {
    type Creator: Creator<Self> + Send + Sync + 'static;

    async fn create_creator(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self::Creator, ApplicationError>;
}

#[async_trait]
impl CreatableItem for UserProfile {
    type Creator = ProfileCreator;

    async fn create_creator(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self::Creator, ApplicationError> {
        ProfileCreator::new(db_handler).await
    }
}

#[async_trait]
impl CreatableItem for ProviderConfig {
    type Creator = ProviderCreator;

    async fn create_creator(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self::Creator, ApplicationError> {
        ProviderCreator::new(db_handler).await
    }
}

#[async_trait]
impl Creator<UserProfile> for ProfileCreator {
    async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        self.handle_key_event(input).await
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        match self.sub_part_creation_state {
            SubPartCreationState::NotCreating => match self.creation_step {
                ProfileCreationStep::EnterName => {
                    self.render_enter_name(f, area)
                }
                ProfileCreationStep::SelectProvider => {
                    self.render_select_provider(f, area)
                }
                ProfileCreationStep::ConfirmCreate => {
                    self.render_confirm_create(f, area)
                }
                ProfileCreationStep::CreatingProfile => {
                    self.render_creating_profile(f, area)
                }
            },
            SubPartCreationState::CreatingProvider(ref mut creator) => {
                creator.render(f, area);
            }
        }
    }

    async fn create_item(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        self.create_profile().await
    }

    fn poll_background_task(&mut self) -> Option<CreatorAction<UserProfile>> {
        self.check_profile_creation_status()
    }
}

#[async_trait]
impl Creator<ProviderConfig> for ProviderCreator {
    async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ProviderConfig>, ApplicationError> {
        self.handle_key_event(input).await
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        match self.current_step {
            ProviderCreationStep::EnterName => self.render_enter_name(f, area),
            ProviderCreationStep::SelectProviderType => {
                self.render_select_provider_type(f, area)
            }
            ProviderCreationStep::SelectModel => {
                self.render_select_model(f, area)
            }
            ProviderCreationStep::ConfigureSettings => {
                self.render_configure_settings(f, area)
            }
            ProviderCreationStep::ConfirmCreate => {
                self.render_confirm_create(f, area)
            }
            ProviderCreationStep::CreatingProvider => {
                self.render_creating_provider(f, area)
            }
        }
    }

    async fn create_item(
        &mut self,
    ) -> Result<CreatorAction<ProviderConfig>, ApplicationError> {
        self.current_step = ProviderCreationStep::CreatingProvider;
        match self.create_provider().await {
            Ok(new_config) => Ok(CreatorAction::Finish(new_config)),
            Err(e) => {
                log::error!("Failed to create provider: {}", e);
                self.current_step = ProviderCreationStep::ConfirmCreate;
                Ok(CreatorAction::Continue)
            }
        }
    }

    fn poll_background_task(
        &mut self,
    ) -> Option<CreatorAction<ProviderConfig>> {
        // No background task to poll
        None
    }
}
