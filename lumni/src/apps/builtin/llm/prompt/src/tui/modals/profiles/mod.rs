mod profile_list;
mod ui_state;

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use profile_list::ProfileList;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph,
};
use ratatui::Frame;
use serde_json::{json, Map, Value};
use tokio::sync::{mpsc, Mutex};
use ui_state::{EditMode, Focus, UIState};

use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, MaskMode, ModalAction,
    ModalWindowTrait, ModalWindowType, ThreadedChatSession,
    UserProfileDbHandler, WindowEvent,
};

pub struct ProfileEditModal {
    profile_list: ProfileList,
    ui_state: UIState,
    db_handler: UserProfileDbHandler,
    settings: Value,
    current_field: usize,
    edit_buffer: String,
    new_key_buffer: String,
    is_new_value_secure: bool,
    predefined_types: Vec<String>,
    selected_type: usize,
    background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    task_start_time: Option<Instant>,
    spinner_state: usize,
    new_profile_name: Option<String>,
}

enum BackgroundTaskResult {
    ProfileCreated(Result<(), ApplicationError>),
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

        let predefined_types = vec![
            "Custom".to_string(),
            "OpenAI".to_string(),
            "Anthropic".to_string(),
        ];

        Ok(Self {
            profile_list,
            ui_state: UIState::new(),
            db_handler,
            settings,
            current_field: 0,
            edit_buffer: String::new(),
            new_key_buffer: String::new(),
            is_new_value_secure: false,
            predefined_types,
            selected_type: 0,
            background_task: None,
            task_start_time: None,
            spinner_state: 0,
            new_profile_name: None,
        })
    }

    fn render_settings_list(&self, f: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = self
            .settings
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
                ) && i == self.current_field
                    && is_editable
                {
                    format!("{}: {}", key, self.edit_buffer)
                } else {
                    let display_value = if is_secure {
                        if self.ui_state.show_secure {
                            value["value"].as_str().unwrap_or("").to_string()
                        } else {
                            "*****".to_string()
                        }
                    } else {
                        value.as_str().unwrap_or("").to_string()
                    };
                    let lock_icon = if is_secure {
                        if self.ui_state.show_secure {
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
                let style = if i == self.current_field
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
            let secure_indicator = if self.is_new_value_secure {
                "🔒 "
            } else {
                ""
            };
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!("{}New key: {}", secure_indicator, self.new_key_buffer),
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )])));
        }

        // Add new value input field if in AddingNewValue mode
        if matches!(self.ui_state.edit_mode, EditMode::AddingNewValue) {
            let secure_indicator = if self.is_new_value_secure {
                "🔒 "
            } else {
                ""
            };
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!(
                    "{}{}: {}",
                    secure_indicator, self.new_key_buffer, self.edit_buffer
                ),
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )])));
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Settings"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let mut state = ListState::default();
        state.select(Some(self.current_field));

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
                    &self.edit_buffer
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
                    self.selected_type = 0;
                } else {
                    self.ui_state.set_focus(Focus::SettingsList);
                    self.load_profile().await?;
                }
            }
            (EditMode::NotEditing, KeyCode::Char('r') | KeyCode::Char('R')) => {
                if !self.profile_list.is_new_profile_selected() {
                    self.edit_buffer = self.profile_list.start_renaming();
                    self.ui_state.set_edit_mode(EditMode::RenamingProfile);
                    self.ui_state.set_focus(Focus::RenamingProfile);
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
                self.profile_list
                    .rename_profile(
                        self.edit_buffer.clone(),
                        &mut self.db_handler,
                    )
                    .await?;
                self.ui_state.set_edit_mode(EditMode::NotEditing);
                self.ui_state.set_focus(Focus::ProfileList);
                self.edit_buffer.clear();
            }
            (EditMode::RenamingProfile, KeyCode::Char(c)) => {
                self.edit_buffer.push(c);
            }
            (EditMode::RenamingProfile, KeyCode::Backspace) => {
                self.edit_buffer.pop();
            }
            (EditMode::RenamingProfile, KeyCode::Esc) => {
                self.ui_state.set_edit_mode(EditMode::NotEditing);
                self.ui_state.set_focus(Focus::ProfileList);
                self.edit_buffer.clear();
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
            .predefined_types
            .iter()
            .enumerate()
            .map(|(i, profile_type)| {
                let style = if i == self.selected_type {
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
        state.select(Some(self.selected_type));

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

    async fn create_new_profile(&mut self) -> Result<(), ApplicationError> {
        let new_profile_name =
            format!("New_Profile_{}", self.profile_list.total_items());
        let profile_type = &self.predefined_types[self.selected_type];

        let mut settings = serde_json::Map::new();
        settings.insert("__PROFILE_TYPE".to_string(), json!(profile_type));

        // Add default settings based on the profile type
        match profile_type.as_str() {
            "OpenAI" => {
                settings.insert("api_key".to_string(), json!(""));
                settings.insert("model".to_string(), json!("gpt-3.5-turbo"));
            }
            "Anthropic" => {
                settings.insert("api_key".to_string(), json!(""));
                settings.insert("model".to_string(), json!("claude-2"));
            }
            "Custom" => {}
            _ => {
                return Err(ApplicationError::InvalidInput(
                    "Unknown profile type".to_string(),
                ))
            }
        }

        let mut db_handler = self.db_handler.clone();
        let (tx, rx) = mpsc::channel(1);

        let new_profile_name_clone = new_profile_name.clone();
        tokio::spawn(async move {
            let result = db_handler
                .create_or_update(&new_profile_name_clone, &json!(settings))
                .await;
            let _ = tx.send(BackgroundTaskResult::ProfileCreated(result)).await;
        });

        self.background_task = Some(rx);
        self.task_start_time = Some(Instant::now());
        self.spinner_state = 0;
        self.ui_state.set_edit_mode(EditMode::CreatingNewProfile);
        self.ui_state.set_focus(Focus::NewProfileType);
        self.new_profile_name = Some(new_profile_name);

        Ok(())
    }

    fn render_activity_indicator(&mut self, frame: &mut Frame, area: Rect) {
        const SPINNER: &[char] =
            &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

        let spinner_char = SPINNER[self.spinner_state];
        self.spinner_state = (self.spinner_state + 1) % SPINNER.len();

        let elapsed = self
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
            let mask_mode = if self.ui_state.show_secure {
                MaskMode::Unmask
            } else {
                MaskMode::Mask
            };
            self.settings = self
                .db_handler
                .get_profile_settings(profile, mask_mode)
                .await?;
            self.current_field = 0;
        }
        Ok(())
    }

    async fn handle_settings_list_input(
        &mut self,
        key_code: KeyCode,
    ) -> Result<WindowEvent, ApplicationError> {
        match (self.ui_state.edit_mode, key_code) {
            (EditMode::NotEditing, KeyCode::Up) => {
                if self.current_field > 0 {
                    self.current_field -= 1;
                }
            }
            (EditMode::NotEditing, KeyCode::Down) => {
                if self.current_field
                    < self.settings.as_object().unwrap().len() - 1
                {
                    self.current_field += 1;
                }
            }
            (EditMode::NotEditing, KeyCode::Enter) => {
                self.start_editing();
            }
            (EditMode::NotEditing, KeyCode::Tab) => {
                self.ui_state.set_focus(Focus::ProfileList);
            }
            (EditMode::NotEditing, KeyCode::Char('s') | KeyCode::Char('S')) => {
                self.ui_state.toggle_secure();
                self.load_profile().await?;
            }
            (EditMode::NotEditing, KeyCode::Char('n')) => {
                self.start_adding_new_value(false);
            }
            (EditMode::NotEditing, KeyCode::Char('N')) => {
                self.start_adding_new_value(true);
            }
            (EditMode::NotEditing, KeyCode::Char('D')) => {
                self.delete_current_key().await?;
            }
            (EditMode::NotEditing, KeyCode::Char('C')) => {
                self.clear_current_key().await?;
            }
            (EditMode::EditingValue, KeyCode::Enter) => {
                self.save_edit().await?;
            }
            (EditMode::EditingValue, KeyCode::Char(c)) => {
                self.edit_buffer.push(c);
            }
            (EditMode::EditingValue, KeyCode::Backspace) => {
                self.edit_buffer.pop();
            }
            (EditMode::AddingNewKey, KeyCode::Enter) => {
                self.confirm_new_key();
            }
            (EditMode::AddingNewKey, KeyCode::Char(c)) => {
                self.new_key_buffer.push(c);
            }
            (EditMode::AddingNewKey, KeyCode::Backspace) => {
                self.new_key_buffer.pop();
            }
            (EditMode::AddingNewValue, KeyCode::Enter) => {
                self.save_edit().await?;
            }
            (EditMode::AddingNewValue, KeyCode::Char(c)) => {
                self.edit_buffer.push(c);
            }
            (EditMode::AddingNewValue, KeyCode::Backspace) => {
                self.edit_buffer.pop();
            }
            (_, KeyCode::Esc) => {
                self.cancel_edit();
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
                if self.selected_type > 0 {
                    self.selected_type -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_type < self.predefined_types.len() - 1 {
                    self.selected_type += 1;
                }
            }
            KeyCode::Enter => {
                self.create_new_profile().await?;
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

    fn start_editing(&mut self) {
        let current_key = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field)
            .unwrap();
        if !current_key.starts_with("__") {
            self.ui_state.set_edit_mode(EditMode::EditingValue);
            self.edit_buffer = self.settings[current_key]
                .as_str()
                .unwrap_or("")
                .to_string();
        }
    }

    fn start_adding_new_value(&mut self, is_secure: bool) {
        self.ui_state.set_edit_mode(EditMode::AddingNewKey);
        self.new_key_buffer.clear();
        self.edit_buffer.clear();
        self.is_new_value_secure = is_secure;
    }

    fn confirm_new_key(&mut self) {
        if !self.new_key_buffer.is_empty() {
            self.ui_state.set_edit_mode(EditMode::AddingNewValue);
        }
    }

    async fn save_edit(&mut self) -> Result<(), ApplicationError> {
        match self.ui_state.edit_mode {
            EditMode::EditingValue => {
                let current_key = self
                    .settings
                    .as_object()
                    .unwrap()
                    .keys()
                    .nth(self.current_field)
                    .unwrap()
                    .to_string();
                self.settings[&current_key] =
                    Value::String(self.edit_buffer.clone());
            }
            EditMode::AddingNewValue => {
                if self.is_new_value_secure {
                    self.settings[&self.new_key_buffer] = json!({
                        "value": self.edit_buffer,
                        "was_encrypted": true
                    });
                } else {
                    self.settings[&self.new_key_buffer] =
                        Value::String(self.edit_buffer.clone());
                }
            }
            _ => return Ok(()),
        }

        if let Some(profile) = self.profile_list.get_selected_profile() {
            self.db_handler
                .create_or_update(profile, &self.settings)
                .await?;
        }

        self.ui_state.set_edit_mode(EditMode::NotEditing);
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
        Ok(())
    }

    fn cancel_edit(&mut self) {
        self.ui_state.set_edit_mode(EditMode::NotEditing);
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
    }

    async fn delete_current_key(&mut self) -> Result<(), ApplicationError> {
        if let Some(current_key) = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field)
        {
            let current_key = current_key.to_string();
            if !current_key.starts_with("__") {
                let mut settings = serde_json::Map::new();
                settings.insert(current_key, Value::Null); // Null indicates deletion
                if let Some(profile) = self.profile_list.get_selected_profile()
                {
                    self.db_handler
                        .create_or_update(profile, &Value::Object(settings))
                        .await?;
                    self.load_profile().await?;
                }
            }
        }
        Ok(())
    }

    async fn clear_current_key(&mut self) -> Result<(), ApplicationError> {
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
                if let Some(profile) = self.profile_list.get_selected_profile()
                {
                    self.db_handler
                        .create_or_update(profile, &self.settings)
                        .await?;
                }
            }
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
        if self.background_task.is_some() {
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
        if let Some(ref mut rx) = self.background_task {
            match rx.try_recv() {
                Ok(BackgroundTaskResult::ProfileCreated(result)) => {
                    self.background_task = None;
                    self.task_start_time = None;
                    match result {
                        Ok(()) => {
                            if let Some(new_profile_name) =
                                self.new_profile_name.take()
                            {
                                self.profile_list.add_profile(new_profile_name);
                                self.load_profile().await?;
                            }
                            self.ui_state.set_edit_mode(EditMode::NotEditing);
                            self.ui_state.set_focus(Focus::SettingsList);
                        }
                        Err(e) => {
                            log::error!("Failed to create profile: {}", e);
                            // Optionally, you could set an error message to display to the user
                        }
                    }
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // Task is still running, continue to show the activity indicator
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Task has ended unexpectedly
                    self.background_task = None;
                    self.task_start_time = None;
                    self.new_profile_name = None;
                    self.ui_state.set_edit_mode(EditMode::NotEditing);
                    self.ui_state.set_focus(Focus::ProfileList);
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
            }
        } else if self.ui_state.edit_mode == EditMode::CreatingNewProfile {
            // If we're in the process of creating a new profile but the background task isn't set,
            // it means we need to start the background task
            self.create_new_profile().await?;
            Ok(WindowEvent::Modal(ModalAction::Refresh))
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
