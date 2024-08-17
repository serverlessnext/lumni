mod new_profile_creator;
mod profile_list;
mod settings_editor;
mod ui_state;

use std::time::Instant;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use new_profile_creator::{BackgroundTaskResult, NewProfileCreator};
use profile_list::ProfileList;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph,
};
use ratatui::Frame;
use serde_json::{json, Map, Value};
use settings_editor::SettingsEditor;
use tokio::sync::{mpsc, Mutex};
use ui_state::{EditMode, Focus, UIState};

use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, MaskMode, ModalAction,
    ModalWindowTrait, ModalWindowType, ThreadedChatSession,
    UserProfileDbHandler, WindowEvent,
};

pub struct ProfileEditModal {
    profile_list: ProfileList,
    settings_editor: SettingsEditor,
    new_profile_creator: NewProfileCreator,
    ui_state: UIState,
    db_handler: UserProfileDbHandler,
    new_profile_name: Option<String>,
}

impl ProfileEditModal {
    pub async fn new(
        mut db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let profiles = db_handler.get_profile_list().await?;
        let profile_list = ProfileList::new(profiles);
        let settings =
            if let Some(profile) = profile_list.get_selected_profile() {
                db_handler
                    .get_profile_settings(profile, MaskMode::Mask)
                    .await?
            } else {
                Value::Object(serde_json::Map::new())
            };
        let settings_editor = SettingsEditor::new(settings);
        let new_profile_creator = NewProfileCreator::new();

        Ok(Self {
            profile_list,
            settings_editor,
            new_profile_creator,
            ui_state: UIState::new(),
            db_handler,
            new_profile_name: None,
        })
    }

    fn render_settings_list(&self, f: &mut Frame, area: Rect) {
        let settings = self.settings_editor.get_settings();
        let mut items: Vec<ListItem> = settings
            .as_object()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, (key, value))| {
                let is_editable = !key.starts_with("__");
                let is_secure = value.is_object()
                    && value.get("was_encrypted") == Some(&Value::Bool(true));
                let content = if matches!(
                    self.ui_state.edit_mode,
                    EditMode::EditingValue
                ) && i
                    == self.settings_editor.get_current_field()
                    && is_editable
                {
                    format!(
                        "{}: {}",
                        key,
                        self.settings_editor.get_edit_buffer()
                    )
                } else {
                    let display_value = if is_secure {
                        if self.settings_editor.is_show_secure() {
                            value["value"].as_str().unwrap_or("").to_string()
                        } else {
                            "*****".to_string()
                        }
                    } else {
                        value.as_str().unwrap_or("").to_string()
                    };
                    let lock_icon = if is_secure {
                        if self.settings_editor.is_show_secure() {
                            "🔓 "
                        } else {
                            "🔒 "
                        }
                    } else {
                        ""
                    };
                    let empty_indicator = if display_value.is_empty() {
                        " (empty)"
                    } else {
                        ""
                    };
                    format!(
                        "{}{}: {}{}",
                        lock_icon, key, display_value, empty_indicator
                    )
                };
                let style = if i == self.settings_editor.get_current_field()
                    && matches!(self.ui_state.focus, Focus::SettingsList)
                {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else if is_editable {
                    Style::default().bg(Color::Black).fg(Color::Cyan)
                } else {
                    Style::default().bg(Color::Black).fg(Color::DarkGray)
                };
                ListItem::new(Line::from(vec![Span::styled(content, style)]))
            })
            .collect();

        // Add new key input field if in AddingNewKey mode
        if matches!(self.ui_state.edit_mode, EditMode::AddingNewKey) {
            let secure_indicator = if self.settings_editor.is_new_value_secure()
            {
                "🔒 "
            } else {
                ""
            };
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!(
                    "{}New key: {}",
                    secure_indicator,
                    self.settings_editor.get_new_key_buffer()
                ),
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )])));
        }

        // Add new value input field if in AddingNewValue mode
        if matches!(self.ui_state.edit_mode, EditMode::AddingNewValue) {
            let secure_indicator = if self.settings_editor.is_new_value_secure()
            {
                "🔒 "
            } else {
                ""
            };
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!(
                    "{}{}: {}",
                    secure_indicator,
                    self.settings_editor.get_new_key_buffer(),
                    self.settings_editor.get_edit_buffer()
                ),
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )])));
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Settings"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let mut state = ListState::default();
        state.select(Some(self.settings_editor.get_current_field()));

        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_profile_list(&self, f: &mut Frame, area: Rect) {
        let profiles = self.profile_list.get_profiles();
        let mut items: Vec<ListItem> = profiles
            .iter()
            .enumerate()
            .map(|(i, profile)| {
                let content = if i == self.profile_list.get_selected_index()
                    && matches!(
                        self.ui_state.edit_mode,
                        EditMode::RenamingProfile
                    ) {
                    self.new_profile_name.as_ref().unwrap_or(profile)
                } else {
                    profile
                };
                let style = if i == self.profile_list.get_selected_index()
                    && matches!(
                        self.ui_state.focus,
                        Focus::ProfileList | Focus::RenamingProfile
                    ) {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else {
                    Style::default().bg(Color::Black).fg(Color::Cyan)
                };
                ListItem::new(Line::from(vec![Span::styled(content, style)]))
            })
            .collect();

        // Add "New Profile" option
        let new_profile_style = if self.profile_list.is_new_profile_selected()
            && matches!(self.ui_state.focus, Focus::ProfileList)
        {
            Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
        } else {
            Style::default().bg(Color::Black).fg(Color::Green)
        };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "+ New Profile",
            new_profile_style,
        )])));

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Profiles"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let mut state = ListState::default();
        state.select(Some(self.profile_list.get_selected_index()));

        f.render_stateful_widget(list, area, &mut state);
    }

    async fn handle_profile_list_input(
        &mut self,
        key_code: KeyCode,
    ) -> Result<WindowEvent, ApplicationError> {
        match (self.ui_state.edit_mode, key_code) {
            (EditMode::NotEditing, KeyCode::Up) => {
                self.profile_list.move_selection_up()
            }
            (EditMode::NotEditing, KeyCode::Down) => {
                self.profile_list.move_selection_down()
            }
            (EditMode::NotEditing, KeyCode::Enter) => {
                if self.profile_list.is_new_profile_selected() {
                    self.ui_state.set_edit_mode(EditMode::CreatingNewProfile);
                    self.ui_state.set_focus(Focus::NewProfileType);
                    self.new_profile_creator.selected_type = 0;
                } else {
                    self.ui_state.set_focus(Focus::SettingsList);
                    self.load_profile().await?;
                }
            }
            (EditMode::NotEditing, KeyCode::Char('r') | KeyCode::Char('R')) => {
                if !self.profile_list.is_new_profile_selected() {
                    self.ui_state.set_edit_mode(EditMode::RenamingProfile);
                    self.ui_state.set_focus(Focus::RenamingProfile);
                    // Use a temporary buffer for renaming
                    self.new_profile_name =
                        Some(self.profile_list.start_renaming());
                }
            }
            (EditMode::NotEditing, KeyCode::Char('D')) => {
                if !self.profile_list.is_new_profile_selected() {
                    self.profile_list
                        .delete_profile(&mut self.db_handler)
                        .await?;
                    self.load_profile().await?;
                }
            }
            (EditMode::NotEditing, KeyCode::Tab) => {
                self.ui_state.set_focus(Focus::SettingsList);
            }
            (EditMode::RenamingProfile, KeyCode::Enter) => {
                if let Some(new_name) = self.new_profile_name.take() {
                    self.profile_list
                        .rename_profile(new_name, &mut self.db_handler)
                        .await?;
                    self.ui_state.set_edit_mode(EditMode::NotEditing);
                    self.ui_state.set_focus(Focus::ProfileList);
                }
            }
            (EditMode::RenamingProfile, KeyCode::Char(c)) => {
                if let Some(ref mut name) = self.new_profile_name {
                    name.push(c);
                }
            }
            (EditMode::RenamingProfile, KeyCode::Backspace) => {
                if let Some(ref mut name) = self.new_profile_name {
                    name.pop();
                }
            }
            (EditMode::RenamingProfile, KeyCode::Esc) => {
                self.new_profile_name = None;
                self.ui_state.set_edit_mode(EditMode::NotEditing);
                self.ui_state.set_focus(Focus::ProfileList);
            }
            (EditMode::NotEditing, KeyCode::Char('q') | KeyCode::Esc) => {
                return Ok(WindowEvent::PromptWindow(None));
            }
            _ => {}
        }

        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
    }

    fn render_new_profile_type(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .new_profile_creator
            .predefined_types
            .iter()
            .enumerate()
            .map(|(i, profile_type)| {
                let style = if i == self.new_profile_creator.selected_type {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else {
                    Style::default().bg(Color::Black).fg(Color::Cyan)
                };
                ListItem::new(Line::from(vec![Span::styled(
                    profile_type,
                    style,
                )]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Profile Type"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let mut state = ListState::default();
        state.select(Some(self.new_profile_creator.selected_type));

        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_instructions(&self, f: &mut Frame, area: Rect) {
        let instructions =
            match (&self.ui_state.focus, &self.ui_state.edit_mode) {
                (Focus::ProfileList, EditMode::NotEditing) => {
                    "↑↓: Navigate | Enter: Select/Create | R: Rename | D: \
                     Delete | Tab: Settings | Esc: Close"
                }
                (Focus::RenamingProfile, EditMode::RenamingProfile) => {
                    "Enter: Confirm Rename | Esc: Cancel"
                }
                (Focus::SettingsList, EditMode::NotEditing) => {
                    "↑↓: Navigate | Enter: Edit | n: New | N: New Secure | D: \
                     Delete | C: Clear | S: Show/Hide Secure | Tab: Profiles | \
                     Esc: Close"
                }
                (Focus::SettingsList, EditMode::EditingValue) => {
                    "Enter: Save | Esc: Cancel"
                }
                (Focus::SettingsList, EditMode::AddingNewKey) => {
                    "Enter: Confirm Key | Esc: Cancel"
                }
                (Focus::SettingsList, EditMode::AddingNewValue) => {
                    "Enter: Save New Value | Esc: Cancel"
                }
                (Focus::NewProfileType, EditMode::CreatingNewProfile) => {
                    "↑↓: Select Type | Enter: Create Profile | Esc: Cancel"
                }
                _ => "",
            };
        let paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(paragraph, area);
    }

    fn render_activity_indicator(&mut self, frame: &mut Frame, area: Rect) {
        const SPINNER: &[char] =
            &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

        let spinner_char = SPINNER[self.new_profile_creator.spinner_state];
        self.new_profile_creator.spinner_state =
            (self.new_profile_creator.spinner_state + 1) % SPINNER.len();

        let elapsed = self
            .new_profile_creator
            .task_start_time
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);
        let content = format!(
            "{} Creating profile... ({} seconds)",
            spinner_char, elapsed
        );

        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(paragraph, area);
    }

    async fn load_profile(&mut self) -> Result<(), ApplicationError> {
        if let Some(profile) = self.profile_list.get_selected_profile() {
            self.settings_editor
                .load_settings(profile, &mut self.db_handler)
                .await?;
        }
        Ok(())
    }

    async fn handle_settings_list_input(
        &mut self,
        key_code: KeyCode,
    ) -> Result<WindowEvent, ApplicationError> {
        match (self.ui_state.edit_mode, key_code) {
            (EditMode::NotEditing, KeyCode::Up) => {
                self.settings_editor.move_selection_up()
            }
            (EditMode::NotEditing, KeyCode::Down) => {
                self.settings_editor.move_selection_down()
            }
            (EditMode::NotEditing, KeyCode::Enter) => {
                if self.settings_editor.start_editing().is_some() {
                    self.ui_state.set_edit_mode(EditMode::EditingValue);
                }
            }
            (EditMode::NotEditing, KeyCode::Tab) => {
                self.ui_state.set_focus(Focus::ProfileList);
            }
            (EditMode::NotEditing, KeyCode::Char('s') | KeyCode::Char('S')) => {
                self.settings_editor.toggle_secure_visibility();
                if let Some(profile) = self.profile_list.get_selected_profile()
                {
                    self.settings_editor
                        .load_settings(profile, &mut self.db_handler)
                        .await?;
                }
            }
            (EditMode::NotEditing, KeyCode::Char('n')) => {
                self.settings_editor.start_adding_new_value(false);
                self.ui_state.set_edit_mode(EditMode::AddingNewKey);
            }
            (EditMode::NotEditing, KeyCode::Char('N')) => {
                self.settings_editor.start_adding_new_value(true);
                self.ui_state.set_edit_mode(EditMode::AddingNewKey);
            }
            (EditMode::NotEditing, KeyCode::Char('D')) => {
                if let Some(profile) = self.profile_list.get_selected_profile()
                {
                    self.settings_editor
                        .delete_current_key(profile, &mut self.db_handler)
                        .await?;
                }
            }
            (EditMode::NotEditing, KeyCode::Char('C')) => {
                if let Some(profile) = self.profile_list.get_selected_profile()
                {
                    self.settings_editor
                        .clear_current_key(profile, &mut self.db_handler)
                        .await?;
                }
            }
            (EditMode::EditingValue, KeyCode::Enter) => {
                if let Some(profile) = self.profile_list.get_selected_profile()
                {
                    self.settings_editor
                        .save_edit(profile, &mut self.db_handler)
                        .await?;
                }
                self.ui_state.set_edit_mode(EditMode::NotEditing);
            }
            (EditMode::EditingValue, KeyCode::Char(c)) => {
                let mut current_value =
                    self.settings_editor.get_edit_buffer().to_string();
                current_value.push(c);
                self.settings_editor.set_edit_buffer(current_value);
            }
            (EditMode::EditingValue, KeyCode::Backspace) => {
                let mut current_value =
                    self.settings_editor.get_edit_buffer().to_string();
                current_value.pop();
                self.settings_editor.set_edit_buffer(current_value);
            }
            (EditMode::AddingNewKey, KeyCode::Enter) => {
                if self.settings_editor.confirm_new_key() {
                    self.ui_state.set_edit_mode(EditMode::AddingNewValue);
                }
            }
            (EditMode::AddingNewKey, KeyCode::Char(c)) => {
                let mut current_value =
                    self.settings_editor.get_new_key_buffer().to_string();
                current_value.push(c);
                self.settings_editor.set_new_key_buffer(current_value);
            }
            (EditMode::AddingNewKey, KeyCode::Backspace) => {
                let mut current_value =
                    self.settings_editor.get_new_key_buffer().to_string();
                current_value.pop();
                self.settings_editor.set_new_key_buffer(current_value);
            }
            (EditMode::AddingNewValue, KeyCode::Enter) => {
                if let Some(profile) = self.profile_list.get_selected_profile()
                {
                    self.settings_editor
                        .save_new_value(profile, &mut self.db_handler)
                        .await?;
                }
                self.ui_state.set_edit_mode(EditMode::NotEditing);
            }
            (EditMode::AddingNewValue, KeyCode::Char(c)) => {
                let mut current_value =
                    self.settings_editor.get_edit_buffer().to_string();
                current_value.push(c);
                self.settings_editor.set_edit_buffer(current_value);
            }
            (EditMode::AddingNewValue, KeyCode::Backspace) => {
                let mut current_value =
                    self.settings_editor.get_edit_buffer().to_string();
                current_value.pop();
                self.settings_editor.set_edit_buffer(current_value);
            }
            (_, KeyCode::Esc) => {
                self.settings_editor.cancel_edit();
                self.ui_state.set_edit_mode(EditMode::NotEditing);
            }
            _ => {}
        }
        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
    }

    async fn handle_new_profile_type_input(
        &mut self,
        key_code: KeyCode,
    ) -> Result<WindowEvent, ApplicationError> {
        match key_code {
            KeyCode::Up => {
                if self.new_profile_creator.selected_type > 0 {
                    self.new_profile_creator.selected_type -= 1;
                }
            }
            KeyCode::Down => {
                if self.new_profile_creator.selected_type
                    < self.new_profile_creator.predefined_types.len() - 1
                {
                    self.new_profile_creator.selected_type += 1;
                }
            }
            KeyCode::Enter => {
                let profile_count = self.profile_list.total_items();
                self.new_profile_creator
                    .create_new_profile(&self.db_handler, profile_count)
                    .await?;
                return Ok(WindowEvent::Modal(ModalAction::Refresh));
            }
            KeyCode::Esc => {
                self.ui_state.set_edit_mode(EditMode::NotEditing);
                self.ui_state.set_focus(Focus::ProfileList);
            }
            _ => {}
        }

        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
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

        let title = Paragraph::new("Profile Editor")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        frame.render_widget(title, main_chunks[0]);

        let content_area = main_chunks[1];

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            .split(content_area);

        self.render_profile_list(frame, content_chunks[0]);

        match self.ui_state.edit_mode {
            EditMode::CreatingNewProfile => {
                self.render_new_profile_type(frame, content_chunks[1])
            }
            _ => self.render_settings_list(frame, content_chunks[1]),
        }

        self.render_instructions(frame, main_chunks[2]);

        // Render activity indicator if a background task is running
        if self.new_profile_creator.background_task.is_some() {
            let indicator_area = Rect {
                x: area.x + 10,
                y: area.bottom() - 3,
                width: area.width - 20,
                height: 3,
            };

            self.render_activity_indicator(frame, indicator_area);
        }
    }

    async fn refresh(&mut self) -> Result<WindowEvent, ApplicationError> {
        if let Some(ref mut rx) = self.new_profile_creator.background_task {
            match rx.try_recv() {
                Ok(BackgroundTaskResult::ProfileCreated(result)) => {
                    self.new_profile_creator.background_task = None;
                    self.new_profile_creator.task_start_time = None;
                    match result {
                        Ok(()) => {
                            if let Some(new_profile_name) =
                                self.new_profile_creator.new_profile_name.take()
                            {
                                self.profile_list.add_profile(new_profile_name);
                                self.load_profile().await?;
                            }
                            self.ui_state.set_edit_mode(EditMode::NotEditing);
                            self.ui_state.set_focus(Focus::SettingsList);
                        }
                        Err(e) => {
                            log::error!("Failed to create profile: {}", e);
                        }
                    }
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.new_profile_creator.background_task = None;
                    self.new_profile_creator.task_start_time = None;
                    self.new_profile_creator.new_profile_name = None;
                    self.ui_state.set_edit_mode(EditMode::NotEditing);
                    self.ui_state.set_focus(Focus::ProfileList);
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
            }
        } else {
            Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
        }
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: &'b mut ThreadedChatSession,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError> {
        let key_code = key_event.current_key().code;

        match self.ui_state.focus {
            Focus::ProfileList | Focus::RenamingProfile => {
                self.handle_profile_list_input(key_code).await
            }
            Focus::SettingsList => {
                self.handle_settings_list_input(key_code).await
            }
            Focus::NewProfileType => {
                self.handle_new_profile_type_input(key_code).await
            }
        }
    }
}
