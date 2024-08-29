use std::time::Instant;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;
use serde_json::{json, Value as JsonValue};
use tokio::sync::mpsc;

use super::profile_edit_renderer::ProfileEditRenderer;
use super::profile_list::ProfileList;
use super::provider_manager::ProviderManagerAction;
use super::settings_editor::{SettingsAction, SettingsEditor};
use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileCreationStep {
    EnterName,
    SelectProvider,
    ConfirmCreate,
    CreatingProfile,
}

#[derive(Debug, Clone)]
pub enum ProfileCreatorAction {
    Refresh,
    WaitForKeyEvent,
    Cancel,
    CreateProfile,
}

pub struct ProfileCreator {
    pub new_profile_name: String,
    pub creation_step: ProfileCreationStep,
    pub provider_manager: ProviderManager,
    pub db_handler: UserProfileDbHandler,
    pub background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    pub task_start_time: Option<Instant>,
    selected_provider: Option<ProviderConfig>,
}

impl ProfileCreator {
    pub fn new(db_handler: UserProfileDbHandler) -> Self {
        Self {
            new_profile_name: String::new(),
            creation_step: ProfileCreationStep::EnterName,
            provider_manager: ProviderManager::new(db_handler.clone()),
            db_handler,
            background_task: None,
            task_start_time: None,
            selected_provider: None,
        }
    }

    pub async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        match self.creation_step {
            ProfileCreationStep::EnterName => {
                self.handle_enter_name(input).await
            }
            ProfileCreationStep::SelectProvider => {
                self.handle_select_provider(input).await
            }
            ProfileCreationStep::ConfirmCreate => {
                self.handle_confirm_create(input)
            }
            ProfileCreationStep::CreatingProfile => {
                Ok(ProfileCreatorAction::WaitForKeyEvent)
            }
        }
    }

    async fn handle_enter_name(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        match input.code {
            KeyCode::Char(c) => {
                self.new_profile_name.push(c);
                Ok(ProfileCreatorAction::Refresh)
            }
            KeyCode::Backspace => {
                self.new_profile_name.pop();
                Ok(ProfileCreatorAction::Refresh)
            }
            KeyCode::Enter => {
                if !self.new_profile_name.is_empty() {
                    self.creation_step = ProfileCreationStep::SelectProvider;
                    self.provider_manager.load_configs().await?;
                    Ok(ProfileCreatorAction::Refresh)
                } else {
                    Ok(ProfileCreatorAction::WaitForKeyEvent)
                }
            }
            KeyCode::Esc => Ok(ProfileCreatorAction::Cancel),
            _ => Ok(ProfileCreatorAction::WaitForKeyEvent),
        }
    }

    async fn handle_select_provider(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        match self.provider_manager.handle_input(input).await? {
            ProviderManagerAction::Refresh => Ok(ProfileCreatorAction::Refresh),
            ProviderManagerAction::ProviderSelected => {
                self.selected_provider =
                    self.provider_manager.get_selected_provider().cloned();
                self.creation_step = ProfileCreationStep::ConfirmCreate;
                Ok(ProfileCreatorAction::Refresh)
            }
            ProviderManagerAction::NoAction => {
                Ok(ProfileCreatorAction::WaitForKeyEvent)
            }
        }
    }

    fn handle_confirm_create(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        match input.code {
            KeyCode::Enter => {
                self.creation_step = ProfileCreationStep::CreatingProfile;
                Ok(ProfileCreatorAction::CreateProfile)
            }
            KeyCode::Esc => {
                self.creation_step = ProfileCreationStep::SelectProvider;
                Ok(ProfileCreatorAction::Refresh)
            }
            _ => Ok(ProfileCreatorAction::WaitForKeyEvent),
        }
    }

    pub async fn create_profile(
        &mut self,
    ) -> Result<UserProfile, ApplicationError> {
        let selected_config = self.selected_provider.as_ref().ok_or(
            ApplicationError::NotReady(
                "No provider config selected".to_string(),
            ),
        )?;

        let mut settings = serde_json::Map::new();
        settings.insert(
            "__TEMPLATE.__MODEL_SERVER".to_string(),
            json!(selected_config.provider_type),
        );
        if let Some(model) = &selected_config.model_identifier {
            settings.insert(
                "__TEMPLATE.MODEL_IDENTIFIER".to_string(),
                json!(model),
            );
        }
        for (key, setting) in &selected_config.additional_settings {
            let value = if setting.is_secure {
                json!({
                    "content": setting.value,
                    "encryption_key": "",
                    "type_info": "string",
                })
            } else {
                json!(setting.value)
            };
            settings.insert(format!("__TEMPLATE.{}", key), value);
        }

        let new_profile = self
            .db_handler
            .create(&self.new_profile_name, &json!(settings))
            .await?;
        Ok(new_profile)
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        match self.creation_step {
            ProfileCreationStep::EnterName => self.render_enter_name(f, area),
            ProfileCreationStep::SelectProvider => {
                self.provider_manager.render(f, area)
            }
            ProfileCreationStep::ConfirmCreate => {
                self.render_confirm_create(f, area)
            }
            ProfileCreationStep::CreatingProfile => {
                self.render_creating_profile(f, area)
            }
        }
    }

    fn render_enter_name(&self, f: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.new_profile_name.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Enter New Profile Name"),
            );
        f.render_widget(input, area);
    }

    fn render_confirm_create(&self, f: &mut Frame, area: Rect) {
        let mut items =
            vec![ListItem::new(format!("Name: {}", self.new_profile_name))];

        if let Some(config) = &self.selected_provider {
            items.push(ListItem::new(format!(
                "Provider: {}",
                config.provider_type
            )));
            if let Some(model) = &config.model_identifier {
                items.push(ListItem::new(format!("Model: {}", model)));
            }
            for (key, setting) in &config.additional_settings {
                items
                    .push(ListItem::new(format!("{}: {}", key, setting.value)));
            }
        }

        let confirm_list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm Profile Creation"),
        );
        f.render_widget(confirm_list, area);
    }

    fn render_creating_profile(&self, f: &mut Frame, area: Rect) {
        let content =
            format!("Creating profile '{}'...", self.new_profile_name);
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Creating Profile"),
            );
        f.render_widget(paragraph, area);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    ProfileList,
    SettingsList,
    ProfileCreation,
    RenamingProfile,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    NotEditing,
    EditingValue,
    AddingNewKey,
    AddingNewValue,
    RenamingProfile,
}

#[derive(Debug)]
pub struct UIState {
    pub focus: Focus,
    pub edit_mode: EditMode,
    pub show_secure: bool,
}

impl UIState {
    pub fn new() -> Self {
        UIState {
            focus: Focus::ProfileList,
            edit_mode: EditMode::NotEditing,
            show_secure: false,
        }
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
    }

    pub fn set_edit_mode(&mut self, mode: EditMode) {
        self.edit_mode = mode;
    }
}

pub struct ProfileEditModal {
    pub profile_list: ProfileList,
    pub settings_editor: SettingsEditor,
    pub ui_state: UIState,
    db_handler: UserProfileDbHandler,
    renderer: ProfileEditRenderer,
    pub profile_creator: Option<ProfileCreator>,
    new_profile_name: Option<String>,
}

impl ProfileEditModal {
    pub async fn new(
        mut db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let profiles = db_handler.list_profiles().await?;
        let default_profile = db_handler.get_default_profile().await?;
        let profile_list = ProfileList::new(profiles, default_profile);

        let settings =
            if let Some(profile) = profile_list.get_selected_profile() {
                db_handler
                    .get_profile_settings(profile, MaskMode::Mask)
                    .await?
            } else {
                JsonValue::Object(serde_json::Map::new())
            };
        let settings_editor = SettingsEditor::new(settings);

        Ok(Self {
            profile_list,
            settings_editor,
            ui_state: UIState::new(),
            db_handler,
            renderer: ProfileEditRenderer::new(),
            profile_creator: None,
            new_profile_name: None,
        })
    }

    async fn set_default_profile(&mut self) -> Result<(), ApplicationError> {
        let selected_profile =
            self.profile_list.get_selected_profile().cloned();
        if let Some(profile) = selected_profile {
            self.db_handler.set_default_profile(&profile).await?;
            self.profile_list.mark_as_default(&profile);
        }
        Ok(())
    }

    async fn rename_profile(
        &mut self,
        new_name: String,
    ) -> Result<(), ApplicationError> {
        if let Some(profile) = self.profile_list.get_selected_profile() {
            self.db_handler.rename_profile(profile, &new_name).await?;
            self.profile_list
                .rename_profile(new_name, &mut self.db_handler)
                .await?;
        }
        self.ui_state.set_edit_mode(EditMode::NotEditing);
        Ok(())
    }

    fn cancel_edit(&mut self) {
        self.settings_editor.cancel_edit();
        self.ui_state.set_edit_mode(EditMode::NotEditing);
    }

    async fn load_selected_profile_settings(
        &mut self,
    ) -> Result<(), ApplicationError> {
        if let Some(profile) = self.profile_list.get_selected_profile() {
            self.settings_editor
                .load_settings(profile, &mut self.db_handler)
                .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl ModalWindowTrait for ProfileEditModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::ProfileEdit
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);
        match self.ui_state.focus {
            Focus::ProfileCreation => {
                if let Some(creator) = &self.profile_creator {
                    creator.render(frame, area);
                }
            }
            _ => self.renderer.render_layout(frame, area, self),
        }
    }

    async fn refresh(&mut self) -> Result<WindowEvent, ApplicationError> {
        if let Some(creator) = &mut self.profile_creator {
            if let Some(ref mut rx) = creator.background_task {
                match rx.try_recv() {
                    Ok(BackgroundTaskResult::ProfileCreated(result)) => {
                        creator.background_task = None;
                        creator.task_start_time = None;
                        match result {
                            Ok(new_profile) => {
                                self.profile_list.add_profile(new_profile);
                                self.profile_creator = None;
                                self.ui_state.focus = Focus::ProfileList;
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
                        self.profile_creator = None;
                        self.ui_state.focus = Focus::ProfileList;
                        return Ok(WindowEvent::Modal(ModalAction::Refresh));
                    }
                }
            }
        }
        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: &'b mut ThreadedChatSession,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError> {
        let key_code = key_event.current_key().code;

        match self.ui_state.focus {
            Focus::ProfileCreation => {
                if let Some(creator) = &mut self.profile_creator {
                    let action = creator
                        .handle_input(key_event.current_key().clone())
                        .await?;
                    match action {
                        ProfileCreatorAction::Refresh => {
                            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                        }
                        ProfileCreatorAction::WaitForKeyEvent => {
                            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                        }
                        ProfileCreatorAction::Cancel => {
                            self.profile_creator = None;
                            self.ui_state.focus = Focus::ProfileList;
                            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                        }
                        ProfileCreatorAction::CreateProfile => {
                            let new_profile = creator.create_profile().await?;
                            self.profile_list.add_profile(new_profile);
                            self.profile_creator = None;
                            self.ui_state.focus = Focus::ProfileList;
                            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                        }
                    }
                } else {
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
            }
            Focus::ProfileList => {
                match key_code {
                    KeyCode::Up => {
                        self.profile_list.move_selection_up();
                        self.load_selected_profile_settings().await?;
                        Ok(WindowEvent::Modal(ModalAction::Refresh))
                    }
                    KeyCode::Down => {
                        self.profile_list.move_selection_down();
                        self.load_selected_profile_settings().await?;
                        Ok(WindowEvent::Modal(ModalAction::Refresh))
                    }
                    KeyCode::Enter => {
                        if self.profile_list.is_new_profile_selected() {
                            // Start new profile creation
                            self.profile_creator = Some(ProfileCreator::new(
                                self.db_handler.clone(),
                            ));
                            self.ui_state.focus = Focus::ProfileCreation;
                            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                        } else {
                            // Handle selecting an existing profile if needed
                            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                        }
                    }
                    KeyCode::Char('n') => {
                        self.profile_creator =
                            Some(ProfileCreator::new(self.db_handler.clone()));
                        self.ui_state.focus = Focus::ProfileCreation;
                        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if let Some(profile) =
                            self.profile_list.get_selected_profile()
                        {
                            self.ui_state
                                .set_edit_mode(EditMode::RenamingProfile);
                            self.new_profile_name = Some(profile.name.clone());
                            Ok(WindowEvent::Modal(ModalAction::Refresh))
                        } else {
                            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                        }
                    }
                    KeyCode::Char(' ') => {
                        self.set_default_profile().await?;
                        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                    }
                    KeyCode::Char('D') => {
                        self.profile_list
                            .delete_profile(&mut self.db_handler)
                            .await?;
                        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                    }
                    KeyCode::Tab => {
                        self.ui_state.set_focus(Focus::SettingsList);
                        if let Some(profile) =
                            self.profile_list.get_selected_profile()
                        {
                            self.settings_editor
                                .load_settings(profile, &mut self.db_handler)
                                .await?;
                        }
                        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                    }
                    KeyCode::Esc => Ok(WindowEvent::PromptWindow(None)),
                    _ => Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent)),
                }
            }
            Focus::RenamingProfile => match key_code {
                KeyCode::Enter => {
                    if let Some(new_name) = self.new_profile_name.take() {
                        self.rename_profile(new_name).await?;
                    }
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut name) = self.new_profile_name {
                        name.push(c);
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Backspace => {
                    if let Some(ref mut name) = self.new_profile_name {
                        name.pop();
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Esc => {
                    self.new_profile_name = None;
                    self.ui_state.set_edit_mode(EditMode::NotEditing);
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                _ => Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent)),
            },
            Focus::SettingsList => {
                let (new_mode, handled, action) = self
                    .settings_editor
                    .handle_key_event(key_code, self.ui_state.edit_mode);

                if handled {
                    if let Some(action) = action {
                        if let Some(profile) =
                            self.profile_list.get_selected_profile()
                        {
                            match action {
                                SettingsAction::ToggleSecureVisibility => {
                                    self.settings_editor
                                        .toggle_secure_visibility(
                                            profile,
                                            &mut self.db_handler,
                                        )
                                        .await?;
                                }
                                SettingsAction::DeleteCurrentKey => {
                                    self.settings_editor
                                        .delete_current_key(
                                            profile,
                                            &mut self.db_handler,
                                        )
                                        .await?;
                                }
                                SettingsAction::ClearCurrentKey => {
                                    self.settings_editor
                                        .clear_current_key(
                                            profile,
                                            &mut self.db_handler,
                                        )
                                        .await?;
                                }
                                SettingsAction::SaveEdit => {
                                    self.settings_editor
                                        .save_edit(
                                            profile,
                                            &mut self.db_handler,
                                        )
                                        .await?;
                                }
                                SettingsAction::SaveNewValue => {
                                    self.settings_editor
                                        .save_new_value(
                                            profile,
                                            &mut self.db_handler,
                                        )
                                        .await?;
                                }
                            }
                        }
                    }
                    self.ui_state.set_edit_mode(new_mode);
                    return Ok(WindowEvent::Modal(ModalAction::Refresh));
                }

                if self.ui_state.edit_mode == EditMode::NotEditing
                    && (key_code == KeyCode::Left
                        || key_code == KeyCode::Char('q')
                        || key_code == KeyCode::Esc
                        || key_code == KeyCode::Tab)
                {
                    self.ui_state.set_focus(Focus::ProfileList);
                    return Ok(WindowEvent::Modal(ModalAction::Refresh));
                }

                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            _ => Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent)),
        }
    }
}
