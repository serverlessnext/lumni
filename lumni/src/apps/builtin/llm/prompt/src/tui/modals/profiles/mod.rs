use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
pub use lumni::Timestamp;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph,
    Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs,
};
use ratatui::Frame;
use serde_json::{json, Value};

use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, MaskMode,
    ModalWindowTrait, ModalWindowType, TextWindowTrait, ThreadedChatSession,
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
}

enum Focus {
    ProfileList,
    SettingsList,
}

enum EditMode {
    NotEditing,
    EditingValue,
    AddingNewKey,
    AddingNewValue,
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
                        format!("{}{}: {}", lock_icon, key, display_value)
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

    fn render_instructions(&self, f: &mut Frame, area: Rect) {
        let instructions = match (&self.focus, &self.edit_mode) {
            (Focus::ProfileList, _) => {
                "↑↓: Navigate | Enter: Select | Tab: Settings | Esc: Close"
            }
            (Focus::SettingsList, EditMode::NotEditing) => {
                "↑↓: Navigate | Enter: Edit | n: New | N: New Secure | S: \
                 Show/Hide Secure | Tab: Profiles | Esc: Close"
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
        };
        let paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(paragraph, area);
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

    fn render_profiles_list(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .profiles
            .iter()
            .enumerate()
            .map(|(i, profile)| {
                let style = if i == self.selected_profile
                    && matches!(self.focus, Focus::ProfileList)
                {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else {
                    Style::default().bg(Color::Black).fg(Color::Cyan)
                };
                ListItem::new(Line::from(vec![Span::styled(profile, style)]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Profiles"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let mut state = ListState::default();
        state.select(Some(self.selected_profile));

        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_selected_profile(&self, f: &mut Frame, area: Rect) {
        let profile = &self.profiles[self.selected_profile];
        let secure_status = if self.show_secure {
            "Visible"
        } else {
            "Hidden"
        };
        let content = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(profile),
            ]),
            Line::from(vec![
                Span::styled(
                    "Secure values: ",
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(secure_status),
            ]),
        ];
        let paragraph = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Selected Profile"),
        );
        f.render_widget(paragraph, area);
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Reduced height for profile details
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            .split(chunks[1]);

        self.render_selected_profile(frame, chunks[0]);
        self.render_profiles_list(frame, main_chunks[0]);
        self.render_settings_list(frame, main_chunks[1]);
        self.render_instructions(frame, chunks[2]);
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: &'b mut ThreadedChatSession,
        _handler: &mut ConversationDbHandler,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match (&self.focus, &self.edit_mode, key_event.current_key().code) {
            // Profile List Navigation
            (Focus::ProfileList, _, KeyCode::Up) => {
                if self.selected_profile > 0 {
                    self.selected_profile -= 1;
                    self.load_profile().await?;
                }
            }
            (Focus::ProfileList, _, KeyCode::Down) => {
                if self.selected_profile < self.profiles.len() - 1 {
                    self.selected_profile += 1;
                    self.load_profile().await?;
                }
            }
            (Focus::ProfileList, _, KeyCode::Enter) => {
                self.focus = Focus::SettingsList;
            }
            (Focus::ProfileList, _, KeyCode::Tab) => {
                self.focus = Focus::SettingsList;
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

            // Global Escape Handling
            (_, _, KeyCode::Esc) => match self.edit_mode {
                EditMode::NotEditing => {
                    return Ok(Some(WindowEvent::PromptWindow(None)))
                }
                _ => self.cancel_edit(),
            },

            // Ignore any other key combinations
            _ => {}
        }

        // Stay in the Modal window
        Ok(Some(WindowEvent::Modal(ModalWindowType::ProfileEdit)))
    }
}
