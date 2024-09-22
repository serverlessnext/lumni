use async_trait::async_trait;
use ratatui::prelude::*;
use serde_json::{json, Value as JsonValue};

use super::provider::ProviderCreationStep;
use super::*;

#[async_trait]
pub trait ManagedItem: Clone + Send + Sync {
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
        db_handler
            .update_configuration_item(&self.into(), settings)
            .await
    }
}

#[async_trait]
impl ManagedItem for DatabaseConfigurationItem {
    async fn delete(
        &self,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        db_handler.delete_configuration_item(self).await
    }

    async fn get_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        db_handler
            .get_configuration_parameters(self, mask_mode)
            .await
    }

    async fn update_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        db_handler.update_configuration_item(self, settings).await
    }
}

#[async_trait]
impl LoadableItem for DatabaseConfigurationItem {
    async fn load_items(
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<Vec<Self>, ApplicationError> {
        db_handler.list_configuration_items("configuration").await
    }

    async fn load_default_item(
        _db_handler: &mut UserProfileDbHandler,
    ) -> Result<Option<Self>, ApplicationError> {
        Ok(None) // DatabaseConfigurationItem doesn't have a default item
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

#[derive(Debug)]
pub enum CreatorAction<T: ManagedItem> {
    Continue, // continue to next step
    Cancel,
    Finish(T),
    LoadAdditionalSettings,
    CreateItem, // spawn a background task to create the item
}

impl<T: ManagedItem> CreatorAction<T> {
    fn map_to_config_item(self) -> CreatorAction<ConfigItem> {
        match self {
            CreatorAction::Continue => CreatorAction::Continue,
            CreatorAction::Cancel => CreatorAction::Cancel,
            CreatorAction::Finish(_) => {
                panic!("Finish action should be handled separately")
            }
            CreatorAction::LoadAdditionalSettings => {
                CreatorAction::LoadAdditionalSettings
            }
            CreatorAction::CreateItem => CreatorAction::CreateItem,
        }
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
impl Creator<ConfigItem> for ProfileCreator {
    async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        let action = self.handle_key_event(input).await?;
        Ok(match action {
            CreatorAction::Continue => CreatorAction::Continue,
            CreatorAction::Cancel => CreatorAction::Cancel,
            CreatorAction::Finish(profile) => {
                CreatorAction::Finish(ConfigItem::UserProfile(profile))
            }
            CreatorAction::LoadAdditionalSettings => {
                CreatorAction::LoadAdditionalSettings
            }
            CreatorAction::CreateItem => CreatorAction::CreateItem,
        })
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        self.render_creator(f, area);
    }

    async fn create_item(
        &mut self,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        let action = self.create_profile().await?;
        Ok(match action {
            CreatorAction::Finish(profile) => {
                CreatorAction::Finish(ConfigItem::UserProfile(profile))
            }
            other => other.map_to_config_item(),
        })
    }

    fn poll_background_task(&mut self) -> Option<CreatorAction<ConfigItem>> {
        self.check_profile_creation_status()
            .map(|action| match action {
                CreatorAction::Finish(profile) => {
                    CreatorAction::Finish(ConfigItem::UserProfile(profile))
                }
                other => other.map_to_config_item(),
            })
    }
}

#[async_trait]
impl Creator<ConfigItem> for ProviderCreator {
    async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        self.handle_key_event(input).await
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        self.render_creator(f, area);
    }

    async fn create_item(
        &mut self,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        self.set_current_step(ProviderCreationStep::CreatingProvider);
        match self.create_provider().await {
            Ok(new_config) => Ok(CreatorAction::Finish(new_config)),
            Err(e) => {
                log::error!("Failed to create provider: {}", e);
                self.set_current_step(ProviderCreationStep::ConfirmCreate);
                Ok(CreatorAction::Continue)
            }
        }
    }

    fn poll_background_task(&mut self) -> Option<CreatorAction<ConfigItem>> {
        None // ProviderCreator doesn't have a background task
    }
}

pub struct ConfigItemManager {
    pub list: SettingsList<ConfigItem>,
    pub settings_editor: SettingsEditor,
    pub creator: Option<Box<dyn Creator<ConfigItem>>>,
    pub rename_buffer: Option<String>,
    pub db_handler: UserProfileDbHandler,
}

impl ConfigItemManager {
    pub async fn new(
        mut db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let profiles = UserProfile::load_items(&mut db_handler).await?;
        let items = profiles.into_iter().map(ConfigItem::UserProfile).collect();
        let default_profile =
            UserProfile::load_default_item(&mut db_handler).await?;
        let list = SettingsList::new(
            items,
            default_profile.map(ConfigItem::UserProfile),
            "Profile".to_string(),
        );

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
        })
    }

    pub async fn refresh_list(
        &mut self,
        current_tab: ConfigTab,
    ) -> Result<WindowMode, ApplicationError> {
        let (items, default_item, item_type) = match current_tab {
            ConfigTab::Profiles => {
                let profiles =
                    UserProfile::load_items(&mut self.db_handler).await?;
                let default_profile =
                    UserProfile::load_default_item(&mut self.db_handler)
                        .await?;
                (
                    profiles.into_iter().map(ConfigItem::UserProfile).collect(),
                    default_profile.map(ConfigItem::UserProfile),
                    "Profile".to_string(),
                )
            }
            ConfigTab::Providers => {
                let providers = self
                    .db_handler
                    .list_configuration_items("provider")
                    .await?;
                (
                    providers
                        .into_iter()
                        .map(|p| {
                            ConfigItem::DatabaseConfig(
                                DatabaseConfigurationItem {
                                    id: p.id,
                                    name: p.name,
                                    section: "provider".to_string(),
                                },
                            )
                        })
                        .collect(),
                    None,
                    "Provider".to_string(),
                )
            }
        };

        self.list = SettingsList::new(items, default_item, item_type);
        self.load_selected_item_settings().await?;
        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
        current_tab: ConfigTab,
    ) -> Result<WindowMode, ApplicationError> {
        match *tab_focus {
            TabFocus::List => {
                self.handle_list_input(key_event, tab_focus, current_tab)
                    .await
            }
            TabFocus::Settings => {
                self.handle_settings_input(key_event, tab_focus).await
            }
            TabFocus::Creation => {
                self.handle_creation_input(key_event, tab_focus, current_tab)
                    .await
            }
        }
    }

    async fn handle_list_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
        current_tab: ConfigTab,
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
                    self.confirm_rename_item().await?;
                } else if self.list.is_new_item_selected() {
                    self.start_item_creation(current_tab).await?;
                    *tab_focus = TabFocus::Creation;
                } else {
                    *tab_focus = TabFocus::Settings;
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Char(' ') => {
                let selected_item = self.list.get_selected_item().cloned();
                if let Some(ConfigItem::UserProfile(profile)) = selected_item {
                    self.list.mark_as_default(&ConfigItem::UserProfile(
                        profile.clone(),
                    ));
                    self.db_handler.set_default_profile(&profile).await?;
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.start_item_renaming();
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Char(c) if self.rename_buffer.is_some() => {
                if let Some(buffer) = &mut self.rename_buffer {
                    buffer.push(c);
                }
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            KeyCode::Backspace => {
                if let Some(buffer) = &mut self.rename_buffer {
                    buffer.pop();
                    Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                } else {
                    self.go_to_previous_step(tab_focus)
                }
            }
            KeyCode::Esc => {
                if self.rename_buffer.is_some() {
                    self.cancel_rename_item();
                    Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                } else {
                    self.go_to_previous_step(tab_focus)
                }
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
        let (new_mode, handled, action) =
            self.settings_editor.handle_key_event(key_event.code);

        if handled {
            self.settings_editor.edit_mode = new_mode;

            if let Some(action) = action {
                let item = self.list.get_selected_item().cloned();

                if let Some(item) = item {
                    match action {
                        SettingsAction::ToggleSecureVisibility => {
                            self.toggle_secure_visibility(&item).await?
                        }
                        SettingsAction::DeleteCurrentKey => {
                            self.delete_current_key(&item).await?
                        }
                        SettingsAction::ClearCurrentKey => {
                            self.clear_current_key(&item).await?
                        }
                        SettingsAction::SaveEdit => {
                            self.save_edit(&item).await?
                        }
                        SettingsAction::SaveNewValue => {
                            self.save_new_value(&item).await?
                        }
                        SettingsAction::ToggleSection => {
                            // Section is already opened in SettingsEditor, just update UI
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
        current_tab: ConfigTab,
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
                            self.refresh_list(current_tab).await?;
                            Ok(WindowMode::Modal(ModalEvent::UpdateUI))
                        }
                        _ => Ok(WindowMode::Modal(
                            ModalEvent::PollBackGroundTask,
                        )),
                    }
                }
                CreatorAction::LoadAdditionalSettings => {
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

    async fn start_item_creation(
        &mut self,
        current_tab: ConfigTab,
    ) -> Result<(), ApplicationError> {
        let creation_type = match current_tab {
            ConfigTab::Profiles => ConfigCreationType::UserProfile,
            ConfigTab::Providers => ConfigCreationType::Provider,
        };
        self.creator = Some(Box::new(
            ConfigItemCreator::new(self.db_handler.clone(), creation_type)
                .await?,
        ));
        Ok(())
    }

    pub fn start_item_renaming(&mut self) {
        if let Some(item) = self.list.get_selected_item() {
            self.rename_buffer =
                Some(<ConfigItem as SettingsItem>::name(item).to_string());
        }
    }

    async fn confirm_rename_item(&mut self) -> Result<(), ApplicationError> {
        if let (Some(new_name), Some(item)) =
            (&self.rename_buffer, self.list.get_selected_item())
        {
            if !new_name.is_empty() {
                self.rename_item(item, new_name).await?;
                self.list.rename_selected_item(new_name.clone());
            }
        }
        self.rename_buffer = None;
        Ok(())
    }

    async fn rename_item(
        &self,
        item: &ConfigItem,
        new_name: &str,
    ) -> Result<(), ApplicationError> {
        match item {
            ConfigItem::UserProfile(profile) => {
                self.db_handler.rename_profile(profile, new_name).await?;
            }
            ConfigItem::DatabaseConfig(config) => {
                self.db_handler
                    .rename_configuration_item(config, new_name)
                    .await?;
            }
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
        item: &ConfigItem,
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
        item: &ConfigItem,
    ) -> Result<(), ApplicationError> {
        let current_field = self.settings_editor.get_current_field();
        if !current_field.starts_with("__") {
            let mut update_settings = JsonValue::Object(serde_json::Map::new());
            // Set the field to null to signal that it should be deleted
            update_settings[current_field] = JsonValue::Null;

            item.update_settings(&mut self.db_handler, &update_settings)
                .await?;

            // Find the next key or set to empty if there are no more keys
            let settings = self.settings_editor.get_settings();
            if let Some(obj) = settings.as_object() {
                let keys: Vec<_> = obj.keys().collect();
                if let Some(pos) =
                    keys.iter().position(|&k| k == &current_field)
                {
                    if pos < keys.len().saturating_sub(1) {
                        self.settings_editor
                            .set_current_field(keys[pos + 1].to_string());
                    } else if pos > 0 {
                        self.settings_editor
                            .set_current_field(keys[pos - 1].to_string());
                    } else {
                        self.settings_editor.set_current_field(String::new());
                    }
                }
            } else {
                self.settings_editor.set_current_field(String::new());
            }

            self.load_selected_item_settings().await?;
        }
        Ok(())
    }

    async fn clear_current_key(
        &mut self,
        item: &ConfigItem,
    ) -> Result<(), ApplicationError> {
        let current_field = self.settings_editor.get_current_field();
        if !current_field.starts_with("__") {
            let mut settings = self.settings_editor.get_settings().clone();
            if let Some(obj) = settings.as_object_mut() {
                obj[current_field] = JsonValue::String("".to_string());
            }
            item.update_settings(&mut self.db_handler, &settings)
                .await?;
            self.load_selected_item_settings().await?;
        }
        Ok(())
    }

    async fn save_edit(
        &mut self,
        item: &ConfigItem,
    ) -> Result<(), ApplicationError> {
        let current_field = self.settings_editor.get_current_field();
        let current_settings = self.settings_editor.get_settings();
        let new_value = self.settings_editor.get_edit_buffer();

        let mut update_settings = JsonValue::Object(serde_json::Map::new());

        if let Some(current_value) = current_settings.get(current_field) {
            if let Some(obj) = current_value.as_object() {
                if obj.contains_key("__encryption_key") {
                    // This is an encrypted value
                    update_settings[current_field] = json!({
                        "__content": new_value,
                        "__encryption_key": "",   // Keep this empty as per your requirement
                        "__type_info": "string",
                    });
                } else {
                    // Update as regular string
                    update_settings[current_field] =
                        JsonValue::String(new_value.to_string());
                }
            } else {
                // Update as regular string
                update_settings[current_field] =
                    JsonValue::String(new_value.to_string());
            }
        }
        item.update_settings(&mut self.db_handler, &update_settings)
            .await?;
        self.load_selected_item_settings().await?;
        Ok(())
    }

    async fn save_new_value(
        &mut self,
        item: &ConfigItem,
    ) -> Result<(), ApplicationError> {
        let new_key = self.settings_editor.get_new_key_buffer().to_string();
        let new_value = self.settings_editor.get_edit_buffer().to_string();
        let mut update_settings = JsonValue::Object(serde_json::Map::new());

        if self.settings_editor.is_new_value_secure() {
            update_settings[&new_key] = json!({
                "__content": new_value,
                "__encryption_key": "",  // Keep this empty as per your requirement
                "__type_info": "string",
            });
        } else {
            update_settings[&new_key] = JsonValue::String(new_value);
        }
        item.update_settings(&mut self.db_handler, &update_settings)
            .await?;
        self.settings_editor.set_current_field(new_key);
        self.load_selected_item_settings().await?;
        Ok(())
    }

    pub fn get_rename_buffer(&self) -> Option<&String> {
        self.rename_buffer.as_ref()
    }

    fn go_to_previous_step(
        &mut self,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowMode, ApplicationError> {
        match tab_focus {
            TabFocus::List => Ok(WindowMode::Conversation(Some(
                ConversationEvent::PromptRead,
            ))),
            TabFocus::Settings => {
                *tab_focus = TabFocus::List;
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
            TabFocus::Creation => {
                self.creator = None;
                *tab_focus = TabFocus::List;
                Ok(WindowMode::Modal(ModalEvent::UpdateUI))
            }
        }
    }

    pub async fn poll_background_task(
        &mut self,
        tab_focus: &mut TabFocus,
        current_tab: ConfigTab,
    ) -> Result<WindowMode, ApplicationError> {
        if let Some(creator) = &mut self.creator {
            if let Some(action) = creator.poll_background_task() {
                match action {
                    CreatorAction::Finish(new_item) => {
                        self.list.add_item(new_item);
                        self.creator = None;
                        *tab_focus = TabFocus::List;
                        self.refresh_list(current_tab).await?;
                        return Ok(WindowMode::Modal(
                            ModalEvent::PollBackGroundTask,
                        ));
                    }
                    CreatorAction::CreateItem => {
                        return Ok(WindowMode::Modal(
                            ModalEvent::PollBackGroundTask,
                        ));
                    }
                    _ => {}
                }
            }
        }
        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
    }

    pub fn render_list(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .list
            .get_items()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let content = if i == self.list.get_selected_index()
                    && self.get_rename_buffer().is_some()
                {
                    self.get_rename_buffer().unwrap().clone()
                } else {
                    item.to_string()
                };

                let style = if i == self.list.get_selected_index() {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else if i == self.list.get_items().len() - 1 {
                    Style::default().fg(Color::Green)
                } else if content.ends_with("(default)") {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                ListItem::new(Span::styled(content, style))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.get_list_title()),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(self.list.get_selected_index()));

        f.render_stateful_widget(list, area, &mut list_state);
    }

    fn get_list_title(&self) -> &str {
        match self.list.get_selected_item() {
            Some(ConfigItem::UserProfile(_)) => "Profiles",
            Some(ConfigItem::DatabaseConfig(config)) => {
                match config.section.as_str() {
                    "provider" => "Providers",
                    "configuration" => "Configurations",
                    _ => "Items",
                }
            }
            None => "Items",
        }
    }

    pub fn render_settings(&self, f: &mut Frame, area: Rect) {
        let item = self.list.get_selected_item();
        let settings_editor = &self.settings_editor;

        if let Some(item) = item {
            let flattened_settings = settings_editor.get_flattened_settings();
            let mut visible_items: Vec<(&String, &JsonValue, &usize)> =
                Vec::new();
            let mut selected_index = 0;
            let mut found_selected = false;

            for (key, value, depth) in flattened_settings.iter() {
                let last_part = key.split('.').last().unwrap_or(key);
                if !last_part.starts_with("__") {
                    if !found_selected && key == &settings_editor.current_field
                    {
                        selected_index = visible_items.len();
                        found_selected = true;
                    }
                    visible_items.push((key, value, depth));
                }
            }

            let mut items: Vec<ListItem> = visible_items
                .into_iter()
                .map(|(key, value, depth)| {
                    let indent = "  ".repeat(*depth);
                    let is_editable = !key.starts_with("__");

                    // Get display name if available, otherwise use the last part of the key
                    let display_name = if let JsonValue::Object(obj) = value {
                        obj.get("__display_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or_else(|| {
                                key.split('.').last().unwrap_or(key)
                            })
                    } else {
                        key.split('.').last().unwrap_or(key)
                    };

                    let key_style = if key == &settings_editor.current_field {
                        Style::default()
                            .bg(Color::Rgb(40, 40, 40))
                            .fg(Color::White)
                    } else {
                        Style::default().fg(Color::Cyan)
                    };

                    let key_span = Span::styled(
                        format!("{}{}: ", indent, display_name),
                        key_style,
                    );

                    let value_span = if settings_editor.edit_mode
                        == EditMode::EditingValue
                        && key == &settings_editor.current_field
                        && is_editable
                    {
                        Span::styled(
                            settings_editor.get_edit_buffer(),
                            Style::default()
                                .bg(Color::Rgb(40, 40, 40))
                                .fg(Color::White),
                        )
                    } else {
                        let mut span =
                            settings_editor.get_display_value_span(value);
                        if span.style.fg == Some(Color::DarkGray) {
                            // dont override style if already set to DarkGray (non-editable)
                        } else if !is_editable {
                            // override style for non-editable fields
                            span.style = Style::default().fg(Color::DarkGray);
                        } else if key == &settings_editor.current_field {
                            // override style for (editable) selected field
                            span.style = Style::default()
                                .bg(Color::Rgb(40, 40, 40))
                                .fg(Color::White);
                        }
                        span
                    };

                    ListItem::new(Line::from(vec![key_span, value_span]))
                })
                .collect();

            // Add new key input field if in AddingNewKey mode
            if settings_editor.edit_mode == EditMode::AddingNewKey {
                let secure_indicator = if settings_editor.is_new_value_secure()
                {
                    "🔒 "
                } else {
                    ""
                };
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!(
                        "{}New key: {}",
                        secure_indicator,
                        settings_editor.get_new_key_buffer()
                    ),
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                )])));
                selected_index = items.len() - 1;
            }

            // Add new value input field if in AddingNewValue mode
            if settings_editor.edit_mode == EditMode::AddingNewValue {
                let secure_indicator = if settings_editor.is_new_value_secure()
                {
                    "🔒 "
                } else {
                    ""
                };
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!(
                        "{}{}: {}",
                        secure_indicator,
                        settings_editor.get_new_key_buffer(),
                        settings_editor.get_edit_buffer()
                    ),
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                )])));
                selected_index = items.len() - 1;
            }

            let title = format!(
                "{} Settings: {}",
                item.item_type(),
                <ConfigItem as SettingsItem>::name(item)
            );
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");

            let mut state = ListState::default();
            state.select(Some(selected_index));

            f.render_stateful_widget(list, area, &mut state);
        } else {
            let paragraph = Paragraph::new("No item selected").block(
                Block::default().borders(Borders::ALL).title("Settings"),
            );
            f.render_widget(paragraph, area);
        }
    }

    pub fn render_content(
        &mut self,
        f: &mut Frame,
        area: Rect,
        tab_focus: TabFocus,
    ) {
        match tab_focus {
            TabFocus::Settings | TabFocus::List => {
                self.render_settings(f, area);
            }
            TabFocus::Creation => {
                if let Some(creator) = &mut self.creator {
                    creator.render(f, area);
                } else {
                    let paragraph = Paragraph::new("No creator available")
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Creation"),
                        );
                    f.render_widget(paragraph, area);
                }
            }
        }
    }
}
