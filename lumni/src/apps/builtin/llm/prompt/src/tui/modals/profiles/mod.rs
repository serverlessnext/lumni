mod new_profile_creator;
mod profile_edit_renderer;
mod profile_list;
mod settings_editor;
mod ui_state;

use std::time::Instant;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use new_profile_creator::{
    BackgroundTaskResult, NewProfileCreator, NewProfileCreatorAction,
};
use profile_edit_renderer::ProfileEditRenderer;
use profile_list::ProfileList;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph,
};
use ratatui::Frame;
use serde_json::{json, Map, Value as JsonValue};
use settings_editor::SettingsEditor;
use tokio::sync::mpsc;
use ui_state::{EditMode, Focus, UIState};

use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, MaskMode, ModalAction,
    ModalWindowTrait, ModalWindowType, ModelServer, ServerTrait,
    ThreadedChatSession, UserProfileDbHandler, WindowEvent,
    SUPPORTED_MODEL_ENDPOINTS,
};

pub struct ProfileEditModal {
    profile_list: ProfileList,
    settings_editor: SettingsEditor,
    ui_state: UIState,
    db_handler: UserProfileDbHandler,
    new_profile_name: Option<String>,
    renderer: ProfileEditRenderer,
}

impl ProfileEditModal {
    pub async fn new(
        mut db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let profiles = db_handler.get_profile_list().await?;
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
            new_profile_name: None,
            renderer: ProfileEditRenderer::new(),
        })
    }

    async fn set_default_profile(&mut self) -> Result<(), ApplicationError> {
        let selected_profile =
            self.profile_list.get_selected_profile().map(String::from);
        if let Some(profile) = selected_profile {
            self.db_handler.set_default_profile(&profile).await?;
            self.profile_list.mark_as_default(&profile);
        }
        Ok(())
    }

    async fn load_profile_or_clear(&mut self) -> Result<(), ApplicationError> {
        if self.profile_list.is_new_profile_selected() {
            self.settings_editor.clear();
            Ok(())
        } else {
            self.load_profile().await
        }
    }

    async fn load_profile(&mut self) -> Result<(), ApplicationError> {
        if let Some(profile) = self.profile_list.get_selected_profile() {
            self.settings_editor
                .load_settings(profile, &mut self.db_handler)
                .await?;
        }
        Ok(())
    }

    fn cancel_edit(&mut self) {
        self.settings_editor.cancel_edit();
        self.ui_state.set_edit_mode(EditMode::NotEditing);
    }
}

#[async_trait]
impl ModalWindowTrait for ProfileEditModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::ProfileEdit
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        self.renderer.render_title(frame, main_chunks[0]);

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            .split(main_chunks[1]);

        self.renderer
            .render_profile_list(frame, content_chunks[0], self);

        match self.ui_state.focus {
            Focus::NewProfileCreation => {
                if let Some(creator) = &self.ui_state.new_profile_creator {
                    creator.render(frame, content_chunks[1]);
                }
            }
            _ => self.renderer.render_settings_list(
                frame,
                content_chunks[1],
                self,
            ),
        }

        self.renderer
            .render_instructions(frame, main_chunks[2], self);
    }

    async fn refresh(&mut self) -> Result<WindowEvent, ApplicationError> {
        if let Some(creator) = &mut self.ui_state.new_profile_creator {
            if let Some(ref mut rx) = creator.background_task {
                match rx.try_recv() {
                    Ok(BackgroundTaskResult::ProfileCreated(result)) => {
                        creator.background_task = None;
                        creator.task_start_time = None;
                        match result {
                            Ok(new_profile_name) => {
                                log::debug!(
                                    "Profile created: {}",
                                    new_profile_name
                                );
                                self.profile_list
                                    .add_profile(new_profile_name.clone());

                                self.load_profile().await?;
                                self.ui_state.cancel_new_profile_creation();
                                self.ui_state.set_focus(Focus::SettingsList);
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
                        self.ui_state.cancel_new_profile_creation();
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
            Focus::ProfileList => match key_code {
                KeyCode::Tab => {
                    if !self.profile_list.is_new_profile_selected() {
                        self.ui_state.set_focus(Focus::SettingsList);
                        self.load_profile().await?;
                    }
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    Ok(WindowEvent::PromptWindow(None))
                }
                KeyCode::Up => {
                    self.profile_list.move_selection_up();
                    self.load_profile_or_clear().await?;
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                KeyCode::Down => {
                    self.profile_list.move_selection_down();
                    self.load_profile_or_clear().await?;
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                KeyCode::Enter => {
                    if self.profile_list.is_new_profile_selected() {
                        self.ui_state.start_new_profile_creation(
                            self.db_handler.clone(),
                        );
                        Ok(WindowEvent::Modal(ModalAction::Refresh))
                    } else {
                        self.ui_state.set_focus(Focus::SettingsList);
                        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                    }
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    if !self.profile_list.is_new_profile_selected() {
                        self.ui_state.set_edit_mode(EditMode::RenamingProfile);
                        self.ui_state.set_focus(Focus::RenamingProfile);
                        self.new_profile_name =
                            Some(self.profile_list.start_renaming());
                        Ok(WindowEvent::Modal(ModalAction::Refresh))
                    } else {
                        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                    }
                }
                KeyCode::Char(' ') => {
                    if !self.profile_list.is_new_profile_selected() {
                        self.set_default_profile().await?;
                        Ok(WindowEvent::Modal(ModalAction::Refresh))
                    } else {
                        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                    }
                }
                KeyCode::Char('D') => {
                    if !self.profile_list.is_new_profile_selected() {
                        self.profile_list
                            .delete_profile(&mut self.db_handler)
                            .await?;
                        self.load_profile_or_clear().await?;
                        Ok(WindowEvent::Modal(ModalAction::Refresh))
                    } else {
                        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                    }
                }
                _ => Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent)),
            },
            Focus::SettingsList => match key_code {
                KeyCode::Left
                | KeyCode::Char('q')
                | KeyCode::Esc
                | KeyCode::Tab => {
                    if self.ui_state.edit_mode == EditMode::NotEditing {
                        self.ui_state.set_focus(Focus::ProfileList);
                    } else {
                        self.cancel_edit();
                    }
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                KeyCode::Up => {
                    self.settings_editor.move_selection_up();
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                KeyCode::Down => {
                    self.settings_editor.move_selection_down();
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                KeyCode::Enter => {
                    match self.ui_state.edit_mode {
                        EditMode::NotEditing => {
                            if self.settings_editor.start_editing().is_some() {
                                self.ui_state
                                    .set_edit_mode(EditMode::EditingValue);
                            }
                        }
                        EditMode::EditingValue => {
                            if let Some(profile) =
                                self.profile_list.get_selected_profile()
                            {
                                self.settings_editor
                                    .save_edit(profile, &mut self.db_handler)
                                    .await?;
                            }
                            self.ui_state.set_edit_mode(EditMode::NotEditing);
                        }
                        EditMode::AddingNewKey => {
                            if self.settings_editor.confirm_new_key() {
                                self.ui_state
                                    .set_edit_mode(EditMode::AddingNewValue);
                            }
                        }
                        EditMode::AddingNewValue => {
                            if let Some(profile) =
                                self.profile_list.get_selected_profile()
                            {
                                self.settings_editor
                                    .save_new_value(
                                        profile,
                                        &mut self.db_handler,
                                    )
                                    .await?;
                            }
                            self.ui_state.set_edit_mode(EditMode::NotEditing);
                        }
                        _ => {}
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.settings_editor.toggle_secure_visibility();
                    if let Some(profile) =
                        self.profile_list.get_selected_profile()
                    {
                        self.settings_editor
                            .load_settings(profile, &mut self.db_handler)
                            .await?;
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Char('n') => {
                    self.settings_editor.start_adding_new_value(false);
                    self.ui_state.set_edit_mode(EditMode::AddingNewKey);
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Char('N') => {
                    self.settings_editor.start_adding_new_value(true);
                    self.ui_state.set_edit_mode(EditMode::AddingNewKey);
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Char('D') => {
                    if let Some(profile) =
                        self.profile_list.get_selected_profile()
                    {
                        self.settings_editor
                            .delete_current_key(profile, &mut self.db_handler)
                            .await?;
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Char('C') => {
                    if let Some(profile) =
                        self.profile_list.get_selected_profile()
                    {
                        self.settings_editor
                            .clear_current_key(profile, &mut self.db_handler)
                            .await?;
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Char(c) => {
                    match self.ui_state.edit_mode {
                        EditMode::EditingValue | EditMode::AddingNewValue => {
                            let mut current_value = self
                                .settings_editor
                                .get_edit_buffer()
                                .to_string();
                            current_value.push(c);
                            self.settings_editor.set_edit_buffer(current_value);
                        }
                        EditMode::AddingNewKey => {
                            let mut current_value = self
                                .settings_editor
                                .get_new_key_buffer()
                                .to_string();
                            current_value.push(c);
                            self.settings_editor
                                .set_new_key_buffer(current_value);
                        }
                        _ => {}
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                KeyCode::Backspace => {
                    match self.ui_state.edit_mode {
                        EditMode::EditingValue | EditMode::AddingNewValue => {
                            let mut current_value = self
                                .settings_editor
                                .get_edit_buffer()
                                .to_string();
                            current_value.pop();
                            self.settings_editor.set_edit_buffer(current_value);
                        }
                        EditMode::AddingNewKey => {
                            let mut current_value = self
                                .settings_editor
                                .get_new_key_buffer()
                                .to_string();
                            current_value.pop();
                            self.settings_editor
                                .set_new_key_buffer(current_value);
                        }
                        _ => {}
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                _ => Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent)),
            },
            Focus::NewProfileCreation => {
                if let Some(creator) = &mut self.ui_state.new_profile_creator {
                    let profile_count = self.profile_list.total_items();
                    if creator.ready_to_create && key_code == KeyCode::Enter {
                        creator.create_new_profile(profile_count).await?;
                        return Ok(WindowEvent::Modal(ModalAction::Refresh));
                    }
                    match creator.handle_input(key_code).await? {
                        NewProfileCreatorAction::Refresh => {
                            Ok(WindowEvent::Modal(ModalAction::Refresh))
                        }
                        NewProfileCreatorAction::WaitForKeyEvent => {
                            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                        }
                        NewProfileCreatorAction::Cancel => {
                            self.ui_state.cancel_new_profile_creation();
                            Ok(WindowEvent::Modal(ModalAction::Refresh))
                        }
                    }
                } else {
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
            }
            Focus::RenamingProfile => match key_code {
                KeyCode::Enter => {
                    if let Some(new_name) = self.new_profile_name.take() {
                        self.profile_list
                            .rename_profile(new_name, &mut self.db_handler)
                            .await?;
                        self.ui_state.set_edit_mode(EditMode::NotEditing);
                        self.ui_state.set_focus(Focus::ProfileList);
                    }
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
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
                    self.ui_state.set_focus(Focus::ProfileList);
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                _ => Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent)),
            },
        }
    }
}
