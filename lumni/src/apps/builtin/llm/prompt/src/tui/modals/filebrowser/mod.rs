use std::path::PathBuf;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyModifiers};
use lumni::{FileType, TableColumnValue};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::widgets::{FileBrowserState, FileBrowserWidget};
use super::{
    ApplicationError, ConversationDbHandler, ConversationEvent, KeyTrack,
    ModalEvent, ModalWindowTrait, ModalWindowType, ThreadedChatSession,
    WindowMode,
};
pub use crate::external as lumni;

const MAX_WIDTH: u16 = 34;

pub struct FileBrowserModal {
    file_browser: FileBrowserWidget,
    file_browser_state: FileBrowserState<'static>,
    selected_file_content: Option<String>,
    selected_file_details: Option<FileDetails>,
}

struct FileDetails {
    name: String,
    size: u64,
    modified: i64,
    is_dir: bool,
}

impl FileBrowserModal {
    pub fn new(base_path: Option<PathBuf>) -> Self {
        let (file_browser, file_browser_state) =
            FileBrowserWidget::new(base_path);
        Self {
            file_browser,
            file_browser_state,
            selected_file_content: None,
            selected_file_details: None,
        }
    }

    pub fn adjust_area(&self, mut area: Rect, max_width: u16) -> Rect {
        area.x = area.width.saturating_sub(max_width);
        area.y = area.y + 0;
        area.width = max_width;
        area.height = area.height.saturating_sub(0);
        area
    }

    fn render_title(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new("📂 File Browser")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(title, area);
    }

    fn render_file_browser(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            &self.file_browser,
            area,
            &mut self.file_browser_state,
        );
    }

    pub fn render_on_frame(&mut self, frame: &mut Frame, mut area: Rect) {
        area = self.adjust_area(area, MAX_WIDTH);
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(Color::Rgb(16, 24, 32)));

        frame.render_widget(modal_block, area);

        let inner_area = area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Min(1),    // File browser
            ])
            .split(inner_area);

        self.render_title(frame, chunks[0]);
        self.render_file_browser(frame, chunks[1]);

        // Render the separator line below the title
        let separator = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            separator,
            Rect::new(inner_area.x, inner_area.y + 1, inner_area.width, 1),
        );
    }

    async fn update_selected_file_info(
        &mut self,
    ) -> Result<(), ApplicationError> {
        if let Some(row) = self
            .file_browser
            .get_selected_table_row(&self.file_browser_state)
        {
            let name = row
                .get_value("name")
                .and_then(|v| match v {
                    TableColumnValue::StringColumn(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            let size = row
                .get_value("size")
                .and_then(|v| match v {
                    TableColumnValue::Uint64Column(n) => Some(*n),
                    _ => None,
                })
                .unwrap_or(0);

            let modified = row
                .get_value("modified")
                .and_then(|v| match v {
                    TableColumnValue::Int64Column(n) => Some(*n),
                    _ => None,
                })
                .unwrap_or(0);

            let file_type = match row.get_value("type") {
                Some(TableColumnValue::Uint8Column(value)) => {
                    FileType::from_u8(*value)
                }
                _ => FileType::Unknown,
            };
            let is_dir = file_type == FileType::Directory;

            self.selected_file_details = Some(FileDetails {
                name: name.clone(),
                size,
                modified,
                is_dir,
            });

            if !is_dir {
                self.selected_file_content =
                    Some(format!("Contents of {}", name));
            } else {
                self.selected_file_content = None;
            }
        } else {
            self.selected_file_details = None;
            self.selected_file_content = None;
        }
        Ok(())
    }
}

#[async_trait]
impl ModalWindowTrait for FileBrowserModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::FileBrowser
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        self.render_on_frame(frame, area);
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: Option<&'b mut ThreadedChatSession>,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError> {
        let current_key = key_event.current_key();
        if current_key.modifiers == KeyModifiers::SHIFT {
            match current_key.code {
                KeyCode::BackTab | KeyCode::Left => {
                    return Ok(WindowMode::Conversation(Some(
                        ConversationEvent::PromptRead,
                    )));
                }
                _ => {}
            }
        }

        let modal_action = self
            .file_browser
            .handle_key_event(key_event, &mut self.file_browser_state)?;

        match key_event.current_key().code {
            KeyCode::Enter | KeyCode::Up | KeyCode::Down => {
                self.update_selected_file_info().await?;
            }
            KeyCode::Esc => {
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::PromptRead,
                )));
            }
            _ => {}
        }
        Ok(WindowMode::Modal(modal_action))
    }

    async fn poll_background_task(
        &mut self,
    ) -> Result<WindowMode, ApplicationError> {
        self.file_browser
            .poll_background_task(&mut self.file_browser_state)
            .await?;
        self.update_selected_file_info().await?;
        Ok(WindowMode::Modal(ModalEvent::PollBackGroundTask))
    }
}
