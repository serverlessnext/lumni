use serde_json::Value as JsonValue;

use super::list::ProviderList;
use super::*;
pub struct ProviderManager {
    pub list: ProviderList,
    pub settings_editor: SettingsEditor,
    pub creator: Option<ProviderCreator>,
    rename_buffer: Option<String>,
    db_handler: UserProfileDbHandler,
    renderer: ProviderEditRenderer,
    pub tab_focus: TabFocus,
}

impl ProviderManager {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let providers = db_handler.load_provider_configs().await?;
        let list = ProviderList::new(providers);

        let settings = if let Some(provider) = list.get_selected_provider() {
            JsonValue::Object(serde_json::Map::from_iter(
                provider.additional_settings.iter().map(|(k, v)| {
                    (k.clone(), JsonValue::String(v.value.clone()))
                }),
            ))
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
            renderer: ProviderEditRenderer::new(),
            tab_focus: TabFocus::List,
        })
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        self.renderer.render_layout(f, area, self);
    }

    pub async fn refresh_provider_list(
        &mut self,
    ) -> Result<WindowEvent, ApplicationError> {
        let providers = self.db_handler.load_provider_configs().await?;
        self.list = ProviderList::new(providers);
        self.load_selected_provider_settings().await?;
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

    async fn handle_list_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowEvent, ApplicationError> {
        match key_event.code {
            KeyCode::Up => {
                if self.rename_buffer.is_some() {
                    self.confirm_rename_provider().await?;
                }
                if self.list.move_selection_up() {
                    self.load_selected_provider_settings().await?;
                }
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Down => {
                if self.rename_buffer.is_some() {
                    self.confirm_rename_provider().await?;
                }
                if self.list.move_selection_down() {
                    self.load_selected_provider_settings().await?;
                }
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Enter => {
                if self.list.is_new_provider_selected() {
                    self.start_provider_creation().await?;
                    *tab_focus = TabFocus::Creation;
                } else if self.rename_buffer.is_some() {
                    self.confirm_rename_provider().await?;
                } else {
                    *tab_focus = TabFocus::Settings;
                }
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.start_provider_renaming();
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
                self.cancel_rename_provider();
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Char('D') => {
                self.delete_selected_provider().await?;
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            _ => Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent)),
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
                // Get the provider outside the mutable borrow
                let provider = self.list.get_selected_provider().cloned();

                if let Some(provider) = provider {
                    match action {
                        SettingsAction::ToggleSecureVisibility => {
                            self.toggle_secure_visibility(&provider).await?;
                        }
                        SettingsAction::DeleteCurrentKey => {
                            self.delete_current_key(&provider).await?;
                        }
                        SettingsAction::ClearCurrentKey => {
                            self.clear_current_key(&provider).await?;
                        }
                        SettingsAction::SaveEdit => {
                            self.save_edit(&provider).await?;
                        }
                        SettingsAction::SaveNewValue => {
                            self.save_new_value(&provider).await?;
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

    async fn handle_creation_input(
        &mut self,
        key_event: KeyEvent,
        tab_focus: &mut TabFocus,
    ) -> Result<WindowEvent, ApplicationError> {
        if let Some(creator) = &mut self.creator {
            match creator.handle_input(key_event).await {
                ProviderCreatorAction::Refresh => {
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProviderCreatorAction::WaitForKeyEvent => {
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                ProviderCreatorAction::Cancel => {
                    self.creator = None;
                    *tab_focus = TabFocus::List;
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProviderCreatorAction::Finish(new_provider) => {
                    self.list.add_provider(new_provider);
                    self.creator = None;
                    *tab_focus = TabFocus::List;
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProviderCreatorAction::LoadModels => {
                    creator.load_models().await?;
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProviderCreatorAction::LoadAdditionalSettings => {
                    let model_server =
                        ModelServer::from_str(&creator.provider_type)?;
                    creator.prepare_additional_settings(&model_server);
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                ProviderCreatorAction::NoAction => {
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
            }
        } else {
            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
        }
    }

    async fn load_selected_provider_settings(
        &mut self,
    ) -> Result<(), ApplicationError> {
        if let Some(provider) = self.list.get_selected_provider() {
            let settings = JsonValue::Object(serde_json::Map::from_iter(
                provider.additional_settings.iter().map(|(k, v)| {
                    (k.clone(), JsonValue::String(v.value.clone()))
                }),
            ));
            self.settings_editor.load_settings(settings);
        } else {
            // Clear settings when "Create new Provider" is selected
            self.settings_editor.clear();
        }
        Ok(())
    }

    async fn start_provider_creation(
        &mut self,
    ) -> Result<(), ApplicationError> {
        self.creator =
            Some(ProviderCreator::new(self.db_handler.clone()).await?);
        Ok(())
    }

    fn start_provider_renaming(&mut self) {
        if let Some(provider) = self.list.get_selected_provider() {
            self.rename_buffer = Some(provider.name.clone());
        }
    }

    async fn confirm_rename_provider(
        &mut self,
    ) -> Result<(), ApplicationError> {
        if let (Some(new_name), Some(provider)) =
            (&self.rename_buffer, self.list.get_selected_provider())
        {
            if !new_name.is_empty() {
                let mut updated_provider = provider.clone();
                updated_provider.name = new_name.clone();
                self.db_handler
                    .save_provider_config(&updated_provider)
                    .await?;
                self.list.rename_selected_provider(new_name.clone());
            }
        }
        self.rename_buffer = None;
        Ok(())
    }

    fn cancel_rename_provider(&mut self) {
        self.rename_buffer = None;
    }

    async fn delete_selected_provider(
        &mut self,
    ) -> Result<(), ApplicationError> {
        self.list.delete_provider(&mut self.db_handler).await?;
        self.load_selected_provider_settings().await?;
        self.rename_buffer = None;
        Ok(())
    }

    pub fn get_rename_buffer(&self) -> Option<&String> {
        self.rename_buffer.as_ref()
    }

    async fn toggle_secure_visibility(
        &mut self,
        provider: &ProviderConfig,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            let mut updated_provider = provider.clone();
            if let Some(setting) =
                updated_provider.additional_settings.get_mut(current_key)
            {
                setting.is_secure = !setting.is_secure;
            }
            self.db_handler
                .save_provider_config(&updated_provider)
                .await?;
            self.load_selected_provider_settings().await?;
        }
        Ok(())
    }

    async fn delete_current_key(
        &mut self,
        provider: &ProviderConfig,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            let mut updated_provider = provider.clone();
            updated_provider.additional_settings.remove(current_key);
            self.db_handler
                .save_provider_config(&updated_provider)
                .await?;
            self.load_selected_provider_settings().await?;
        }
        Ok(())
    }

    async fn clear_current_key(
        &mut self,
        provider: &ProviderConfig,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            let mut updated_provider = provider.clone();
            if let Some(setting) =
                updated_provider.additional_settings.get_mut(current_key)
            {
                setting.value.clear();
            }
            self.db_handler
                .save_provider_config(&updated_provider)
                .await?;
            self.load_selected_provider_settings().await?;
        }
        Ok(())
    }

    async fn save_edit(
        &mut self,
        provider: &ProviderConfig,
    ) -> Result<(), ApplicationError> {
        if let Some(current_key) = self.settings_editor.get_current_key() {
            let mut updated_provider = provider.clone();
            if let Some(setting) =
                updated_provider.additional_settings.get_mut(current_key)
            {
                setting.value =
                    self.settings_editor.get_edit_buffer().to_string();
            }
            self.db_handler
                .save_provider_config(&updated_provider)
                .await?;
            self.load_selected_provider_settings().await?;
        }
        Ok(())
    }

    async fn save_new_value(
        &mut self,
        provider: &ProviderConfig,
    ) -> Result<(), ApplicationError> {
        let new_key = self.settings_editor.get_new_key_buffer().to_string();
        let new_value = self.settings_editor.get_edit_buffer().to_string();

        let mut updated_provider = provider.clone();
        updated_provider.additional_settings.insert(
            new_key.clone(),
            ProviderConfigOptions {
                name: new_key.clone(),
                display_name: new_key.clone(),
                value: new_value,
                is_secure: false,
                placeholder: String::new(),
            },
        );

        self.db_handler
            .save_provider_config(&updated_provider)
            .await?;
        self.load_selected_provider_settings().await?;

        Ok(())
    }
}
