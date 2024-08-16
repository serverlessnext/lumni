use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph,
};
use ratatui::Frame;
use serde_json::{json, Map, Value};
use tokio::sync::{mpsc, Mutex};

use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, MaskMode, ModalAction,
    ModalWindowTrait, ModalWindowType, ThreadedChatSession,
    UserProfileDbHandler, WindowEvent,
};
pub use crate::external as lumni;

pub struct ProfileEditModal {
    profiles: Vec<String>,
    selected_profile: usize,
    settings: Value,
    current_field: usize,
    edit_mode: EditMode,
    edit_buffer: String,
    new_key_buffer: String,
    is_new_value_secure: bool,
    db_handler: UserProfileDbHandler,
    focus: Focus,
    show_secure: bool,
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
enum Focus {
    ProfileList,
    SettingsList,
    NewProfileType,
    RenamingProfile,
}

enum EditMode {
    NotEditing,
    EditingValue,
    AddingNewKey,
    AddingNewValue,
    CreatingNewProfile,
    RenamingProfile,
}

impl ProfileEditModal {
    pub async fn new(
        mut db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let profiles = db_handler.get_profile_list().await?;
        let selected_profile = 0;
        let settings = if !profiles.is_empty() {
            db_handler
                .get_profile_settings(&profiles[0], MaskMode::Mask)
                .await?
        } else {
            Value::Object(serde_json::Map::new())
        };

        // Define predefined types
        let predefined_types = vec![
            "Custom".to_string(),
            "OpenAI".to_string(),
            "Anthropic".to_string(),
            // Add more predefined types as needed
        ];

        Ok(Self {
            profiles,
            selected_profile,
            settings,
            current_field: 0,
            edit_mode: EditMode::NotEditing,
            edit_buffer: String::new(),
            new_key_buffer: String::new(),
            is_new_value_secure: false,
            db_handler,
            focus: Focus::ProfileList,
            show_secure: false,
            predefined_types,
            selected_type: 0,
            background_task: None,
            task_start_time: None,
            spinner_state: 0,
            new_profile_name: None,
        })
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
                let mut settings = Map::new();
                settings.insert(current_key, Value::Null); // Null indicates deletion
                let profile = &self.profiles[self.selected_profile];
                self.db_handler
                    .create_or_update(profile, &Value::Object(settings))
                    .await?;
                self.load_profile().await?;
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
                let profile = &self.profiles[self.selected_profile];
                self.db_handler
                    .create_or_update(profile, &self.settings)
                    .await?;
            }
        }
        Ok(())
    }

    async fn delete_current_profile(&mut self) -> Result<(), ApplicationError> {
        if !self.profiles.is_empty() {
            let profile_name = &self.profiles[self.selected_profile];
            self.db_handler.delete_profile(profile_name).await?;
            self.profiles.remove(self.selected_profile);
            if self.selected_profile >= self.profiles.len()
                && !self.profiles.is_empty()
            {
                self.selected_profile = self.profiles.len() - 1;
            }
            if !self.profiles.is_empty() {
                self.load_profile().await?;
            } else {
                self.settings = Value::Object(Map::new());
            }
        }
        Ok(())
    }

    fn start_renaming_profile(&mut self) {
        if !self.profiles.is_empty() {
            self.edit_mode = EditMode::RenamingProfile;
            self.focus = Focus::RenamingProfile;
            self.edit_buffer = self.profiles[self.selected_profile].clone();
        }
    }

    async fn confirm_rename_profile(&mut self) -> Result<(), ApplicationError> {
        if !self.profiles.is_empty() && !self.edit_buffer.trim().is_empty() {
            let old_name = &self.profiles[self.selected_profile];
            let new_name = self.edit_buffer.trim().to_string();
            if old_name != &new_name {
                self.db_handler.rename_profile(old_name, &new_name).await?;
                self.profiles[self.selected_profile] = new_name;
            }
        }
        self.exit_rename_mode();
        Ok(())
    }

    fn exit_rename_mode(&mut self) {
        self.edit_mode = EditMode::NotEditing;
        self.focus = Focus::ProfileList;
        self.edit_buffer.clear();
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
                let content =
                    if matches!(self.edit_mode, EditMode::EditingValue)
                        && i == self.current_field
                        && is_editable
                    {
                        format!("{}: {}", key, self.edit_buffer)
                    } else {
                        let display_value = if is_secure {
                            if self.show_secure {
                                value["value"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string()
                            } else {
                                "*****".to_string()
                            }
                        } else {
                            value.as_str().unwrap_or("").to_string()
                        };
                        let lock_icon = if is_secure {
                            if self.show_secure {
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
                    && matches!(self.focus, Focus::SettingsList)
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
        if matches!(self.edit_mode, EditMode::AddingNewKey) {
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
        if matches!(self.edit_mode, EditMode::AddingNewValue) {
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
        let mut items: Vec<ListItem> = self
            .profiles
            .iter()
            .enumerate()
            .map(|(i, profile)| {
                let content = if i == self.selected_profile
                    && matches!(self.edit_mode, EditMode::RenamingProfile)
                {
                    self.edit_buffer.clone()
                } else {
                    profile.clone()
                };
                let style = if i == self.selected_profile
                    && matches!(
                        self.focus,
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
        let new_profile_style = if self.selected_profile == self.profiles.len()
            && matches!(self.focus, Focus::ProfileList)
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
        state.select(Some(self.selected_profile));

        f.render_stateful_widget(list, area, &mut state);
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
        let instructions = match (&self.focus, &self.edit_mode) {
            (Focus::ProfileList, EditMode::NotEditing) => {
                "↑↓: Navigate | Enter: Select/Create | R: Rename | D: Delete | \
                 Tab: Settings | Esc: Close"
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
            format!("New_Profile_{}", self.profiles.len() + 1);
        let profile_type = self.predefined_types[self.selected_type].clone();

        let mut settings = Map::new();
        settings.insert(
            "__PROFILE_TYPE".to_string(),
            Value::String(profile_type.clone()),
        );

        // Add default settings based on the profile type
        match profile_type.as_str() {
            "OpenAI" => {
                settings.insert(
                    "api_key".to_string(),
                    Value::String("".to_string()),
                );
                settings.insert(
                    "model".to_string(),
                    Value::String("gpt-3.5-turbo".to_string()),
                );
            }
            "Anthropic" => {
                settings.insert(
                    "api_key".to_string(),
                    Value::String("".to_string()),
                );
                settings.insert(
                    "model".to_string(),
                    Value::String("claude-2".to_string()),
                );
            }
            "Custom" => {
                // No default settings for custom profiles
            }
            _ => {
                // Handle unexpected profile types
                return Err(ApplicationError::InvalidInput(
                    "Unknown profile type".to_string(),
                ));
            }
        }

        let db_handler = Arc::new(Mutex::new(self.db_handler.clone()));
        let (tx, rx) = mpsc::channel(1);

        let new_profile_name_clone = new_profile_name.clone();
        tokio::spawn(async move {
            let result = db_handler
                .lock()
                .await
                .create_or_update(
                    &new_profile_name_clone,
                    &Value::Object(settings),
                )
                .await;
            let _ = tx.send(BackgroundTaskResult::ProfileCreated(result)).await;
        });

        self.background_task = Some(rx);
        self.task_start_time = Some(std::time::Instant::now());
        self.spinner_state = 0;
        self.edit_mode = EditMode::CreatingNewProfile;
        self.focus = Focus::NewProfileType;
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

    fn start_adding_new_value(&mut self, is_secure: bool) {
        self.edit_mode = EditMode::AddingNewKey;
        self.new_key_buffer.clear();
        self.edit_buffer.clear();
        self.is_new_value_secure = is_secure;
    }

    fn confirm_new_key(&mut self) {
        if !self.new_key_buffer.is_empty() {
            self.edit_mode = EditMode::AddingNewValue;
        }
    }

    async fn save_edit(&mut self) -> Result<(), ApplicationError> {
        match self.edit_mode {
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

        let profile = &self.profiles[self.selected_profile];
        self.db_handler
            .create_or_update(profile, &self.settings)
            .await?;

        self.edit_mode = EditMode::NotEditing;
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
        Ok(())
    }

    fn cancel_edit(&mut self) {
        self.edit_mode = EditMode::NotEditing;
        self.edit_buffer.clear();
        self.new_key_buffer.clear();
        self.is_new_value_secure = false;
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
            self.edit_mode = EditMode::EditingValue;
            self.edit_buffer = self.settings[current_key]
                .as_str()
                .unwrap_or("")
                .to_string();
        }
    }

    async fn load_profile(&mut self) -> Result<(), ApplicationError> {
        let profile = &self.profiles[self.selected_profile];
        let mask_mode = if self.show_secure {
            MaskMode::Unmask
        } else {
            MaskMode::Mask
        };
        self.settings = self
            .db_handler
            .get_profile_settings(profile, mask_mode)
            .await?;
        self.current_field = 0;
        Ok(())
    }

    async fn toggle_secure_visibility(
        &mut self,
    ) -> Result<(), ApplicationError> {
        self.show_secure = !self.show_secure;
        self.load_profile().await
    }

    fn move_selection_up(&mut self) {
        if self.current_field > 0 {
            self.current_field -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        if self.current_field < self.settings.as_object().unwrap().len() - 1 {
            self.current_field += 1;
        }
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

        match self.edit_mode {
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
                                self.profiles.push(new_profile_name);
                                self.selected_profile = self.profiles.len() - 1;
                                self.load_profile().await?;
                            }
                            self.edit_mode = EditMode::NotEditing;
                            self.focus = Focus::SettingsList;
                        }
                        Err(e) => {
                            log::error!("Failed to create profile: {}", e);
                        }
                    }
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // Task is still running, update will happen in render_activity_indicator
                    Ok(WindowEvent::Modal(ModalAction::Refresh))
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Task has ended unexpectedly
                    self.background_task = None;
                    self.task_start_time = None;
                    self.new_profile_name = None;
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
        match (&self.focus, &self.edit_mode, key_event.current_key().code) {
            // Profile List Navigation and Actions
            (Focus::ProfileList, EditMode::NotEditing, KeyCode::Up) => {
                if self.selected_profile > 0 {
                    self.selected_profile -= 1;
                    self.load_profile().await?;
                }
            }
            (Focus::ProfileList, EditMode::NotEditing, KeyCode::Down) => {
                if self.selected_profile < self.profiles.len() {
                    self.selected_profile += 1;
                    if self.selected_profile < self.profiles.len() {
                        self.load_profile().await?;
                    }
                }
            }
            (Focus::ProfileList, EditMode::NotEditing, KeyCode::Enter) => {
                if self.selected_profile == self.profiles.len() {
                    // "New Profile" option selected
                    self.edit_mode = EditMode::CreatingNewProfile;
                    self.focus = Focus::NewProfileType;
                    self.selected_type = 0;
                } else {
                    self.focus = Focus::SettingsList;
                }
            }
            (
                Focus::ProfileList,
                EditMode::NotEditing,
                KeyCode::Char('r') | KeyCode::Char('R'),
            ) => {
                self.start_renaming_profile();
            }
            (Focus::ProfileList, EditMode::NotEditing, KeyCode::Char('D')) => {
                self.delete_current_profile().await?;
            }
            (Focus::ProfileList, EditMode::NotEditing, KeyCode::Tab) => {
                self.focus = Focus::SettingsList;
            }

            // Renaming Profile
            (
                Focus::RenamingProfile,
                EditMode::RenamingProfile,
                KeyCode::Enter | KeyCode::Up | KeyCode::Down | KeyCode::Tab,
            ) => {
                self.confirm_rename_profile().await?;
                match key_event.current_key().code {
                    KeyCode::Up => {
                        if self.selected_profile > 0 {
                            self.selected_profile -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected_profile < self.profiles.len() - 1 {
                            self.selected_profile += 1;
                        }
                    }
                    KeyCode::Tab => {
                        self.focus = Focus::SettingsList;
                    }
                    _ => {}
                }
            }
            (
                Focus::RenamingProfile,
                EditMode::RenamingProfile,
                KeyCode::Char(c),
            ) => {
                self.edit_buffer.push(c);
            }
            (
                Focus::RenamingProfile,
                EditMode::RenamingProfile,
                KeyCode::Backspace,
            ) => {
                self.edit_buffer.pop();
            }

            // New Profile Type Selection
            (
                Focus::NewProfileType,
                EditMode::CreatingNewProfile,
                KeyCode::Up,
            ) => {
                if self.selected_type > 0 {
                    self.selected_type -= 1;
                }
            }
            (
                Focus::NewProfileType,
                EditMode::CreatingNewProfile,
                KeyCode::Down,
            ) => {
                if self.selected_type < self.predefined_types.len() - 1 {
                    self.selected_type += 1;
                }
            }
            (
                Focus::NewProfileType,
                EditMode::CreatingNewProfile,
                KeyCode::Enter,
            ) => {
                self.create_new_profile().await?;
                return Ok(WindowEvent::Modal(ModalAction::Refresh));
            }

            // Settings List Navigation and Editing
            (Focus::SettingsList, EditMode::NotEditing, KeyCode::Up) => {
                self.move_selection_up()
            }
            (Focus::SettingsList, EditMode::NotEditing, KeyCode::Down) => {
                self.move_selection_down()
            }
            (Focus::SettingsList, EditMode::NotEditing, KeyCode::Enter) => {
                self.start_editing()
            }
            (Focus::SettingsList, EditMode::NotEditing, KeyCode::Tab) => {
                self.focus = Focus::ProfileList;
            }
            (
                Focus::SettingsList,
                EditMode::NotEditing,
                KeyCode::Char('s') | KeyCode::Char('S'),
            ) => {
                self.toggle_secure_visibility().await?;
            }
            (Focus::SettingsList, EditMode::NotEditing, KeyCode::Char('n')) => {
                self.start_adding_new_value(false);
            }
            (Focus::SettingsList, EditMode::NotEditing, KeyCode::Char('N')) => {
                self.start_adding_new_value(true);
            }

            // Editing Existing Value
            (Focus::SettingsList, EditMode::EditingValue, KeyCode::Enter) => {
                self.save_edit().await?
            }
            (Focus::SettingsList, EditMode::EditingValue, KeyCode::Char(c)) => {
                self.edit_buffer.push(c)
            }
            (
                Focus::SettingsList,
                EditMode::EditingValue,
                KeyCode::Backspace,
            ) => {
                self.edit_buffer.pop();
            }

            // Adding New Key
            (Focus::SettingsList, EditMode::AddingNewKey, KeyCode::Enter) => {
                self.confirm_new_key();
            }
            (Focus::SettingsList, EditMode::AddingNewKey, KeyCode::Char(c)) => {
                self.new_key_buffer.push(c);
            }
            (
                Focus::SettingsList,
                EditMode::AddingNewKey,
                KeyCode::Backspace,
            ) => {
                self.new_key_buffer.pop();
            }

            // Adding New Value
            (Focus::SettingsList, EditMode::AddingNewValue, KeyCode::Enter) => {
                self.save_edit().await?;
            }
            (
                Focus::SettingsList,
                EditMode::AddingNewValue,
                KeyCode::Char(c),
            ) => {
                self.edit_buffer.push(c);
            }
            (
                Focus::SettingsList,
                EditMode::AddingNewValue,
                KeyCode::Backspace,
            ) => {
                self.edit_buffer.pop();
            }

            // Delete and clear
            (Focus::SettingsList, EditMode::NotEditing, KeyCode::Char('D')) => {
                self.delete_current_key().await?;
            }
            (Focus::SettingsList, EditMode::NotEditing, KeyCode::Char('C')) => {
                self.clear_current_key().await?;
            }

            // Global Escape Handling
            (_, _, KeyCode::Esc) => match self.edit_mode {
                EditMode::NotEditing => {
                    return Ok(WindowEvent::PromptWindow(None))
                }
                EditMode::RenamingProfile => {
                    self.edit_mode = EditMode::NotEditing;
                    self.focus = Focus::ProfileList;
                    self.edit_buffer.clear();
                }
                _ => self.cancel_edit(),
            },

            // Ignore any other key combinations
            _ => {}
        }

        // Stay in the Modal window, waiting for next key event
        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
    }
}
