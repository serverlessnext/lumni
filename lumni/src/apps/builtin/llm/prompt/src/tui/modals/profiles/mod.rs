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
use serde_json::Value;

use super::{
    ApplicationError, ConversationDbHandler,
    KeyTrack, MaskMode,
    ModalWindowTrait, ModalWindowType, TextWindowTrait,
    ThreadedChatSession, UserProfileDbHandler, WindowEvent,
};
pub use crate::external as lumni;

pub struct ProfileEditModal {
    settings: Value,
    current_field: usize,
    editing: bool,
    edit_buffer: String,
    db_handler: UserProfileDbHandler,
}

impl ProfileEditModal {
    pub async fn new(
        mut db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        eprintln!("db_handler: {:?}", db_handler);
        let settings = db_handler
            .get_profile_settings("foo", MaskMode::Unmask)
            .await?;
        eprintln!("settings: {:?}", settings);
        Ok(Self {
            settings,
            current_field: 0,
            editing: false,
            edit_buffer: String::new(),
            db_handler,
        })
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("Profile Edit")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn render_settings_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .settings
            .as_object()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, (key, value))| {
                let content = if self.editing && i == self.current_field {
                    format!("{}: {}", key, self.edit_buffer)
                } else {
                    format!("{}: {}", key, value.as_str().unwrap_or(""))
                };
                let style = if i == self.current_field {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![Span::styled(content, style)]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Settings"));

        let mut state = ListState::default();
        state.select(Some(self.current_field));

        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_instructions(&self, f: &mut Frame, area: Rect) {
        let instructions = if self.editing {
            "Enter: Save | Esc: Cancel"
        } else {
            "↑↓: Navigate | Enter: Edit | Esc: Close"
        };
        let paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(paragraph, area);
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

    fn start_editing(&mut self) {
        self.editing = true;
        let current_key = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field)
            .unwrap();
        self.edit_buffer = self.settings[current_key]
            .as_str()
            .unwrap_or("")
            .to_string();
    }

    async fn save_edit(&mut self) -> Result<(), ApplicationError> {
        self.editing = false;
        let current_key = self
            .settings
            .as_object()
            .unwrap()
            .keys()
            .nth(self.current_field)
            .unwrap()
            .to_string();
        self.settings[&current_key] = Value::String(self.edit_buffer.clone());
        self.db_handler
            .create_or_update("foo", &self.settings)
            .await?;
        Ok(())
    }

    fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }
}

#[async_trait]
impl ModalWindowTrait for ProfileEditModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::ProfileEdit
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        frame.render_widget(Clear, area);
        self.render_title(frame, chunks[0]);
        self.render_settings_list(frame, chunks[1]);
        self.render_instructions(frame, chunks[2]);
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: &'b mut ThreadedChatSession,
        _handler: &mut ConversationDbHandler,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match (self.editing, key_event.current_key().code) {
            (false, KeyCode::Up) => self.move_selection_up(),
            (false, KeyCode::Down) => self.move_selection_down(),
            (false, KeyCode::Enter) => self.start_editing(),
            (false, KeyCode::Esc) => {
                return Ok(Some(WindowEvent::PromptWindow(None)))
            }
            (true, KeyCode::Enter) => self.save_edit().await?,
            (true, KeyCode::Esc) => self.cancel_edit(),
            (true, KeyCode::Char(c)) => self.edit_buffer.push(c),
            (true, KeyCode::Backspace) => {
                self.edit_buffer.pop();
            }
            _ => {}
        }
        Ok(Some(WindowEvent::Modal(ModalWindowType::ProfileEdit)))
    }
}
