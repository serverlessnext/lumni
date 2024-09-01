use std::path::PathBuf;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use lumni::{FileType, TableColumnValue};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{
    ApplicationError, ConversationDbHandler, FileBrowserWidget, KeyTrack,
    ModalAction, ModalWindowTrait, ModalWindowType, ThreadedChatSession,
    WindowEvent,
};
pub use crate::external as lumni;

pub struct FileBrowserModal<'a> {
    file_browser: FileBrowserWidget<'a>,
    selected_file_content: Option<String>,
    selected_file_details: Option<FileDetails>,
}

struct FileDetails {
    name: String,
    size: u64,
    modified: i64,
    is_dir: bool,
}

impl<'a> FileBrowserModal<'a> {
    pub fn new(base_path: Option<PathBuf>) -> Self {
        Self {
            file_browser: FileBrowserWidget::new(base_path),
            selected_file_content: None,
            selected_file_details: None,
        }
    }

    fn render_file_details(&self, frame: &mut Frame, area: Rect) {
        let details = match &self.selected_file_details {
            Some(details) => vec![
                Line::from(Span::raw(format!("Name: {}", details.name))),
                Line::from(Span::raw(format!(
                    "Type: {}",
                    if details.is_dir { "Directory" } else { "File" }
                ))),
                Line::from(Span::raw(format!("Size: {} bytes", details.size))),
                Line::from(Span::raw(format!(
                    "Modified: {}",
                    details.modified
                ))),
            ],
            None => vec![Line::from(Span::raw("No file selected"))],
        };

        let paragraph = Paragraph::new(details).block(
            Block::default().title("File Details").borders(Borders::ALL),
        );
        frame.render_widget(paragraph, area);
    }

    fn render_file_content(&self, frame: &mut Frame, area: Rect) {
        let content = match &self.selected_file_content {
            Some(content) => content,
            None => "No file selected",
        };
        let paragraph = Paragraph::new(content).block(
            Block::default().title("File Content").borders(Borders::ALL),
        );
        frame.render_widget(paragraph, area);
    }

    fn render_instructions(&self, frame: &mut Frame, area: Rect) {
        let instructions = "↑↓: Navigate | Enter: Open | Backspace: Go Up | \
                            Tab: Switch Focus | Esc: Close";
        let paragraph = Paragraph::new(Span::raw(instructions))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(paragraph, area);
    }

    async fn update_selected_file_info(
        &mut self,
    ) -> Result<(), ApplicationError> {
        if let Some(row) = &self.file_browser.get_selected_table_row() {
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
                // TODO: placeholder
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
impl ModalWindowTrait for FileBrowserModal<'_> {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::FileBrowser
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Main content
                Constraint::Length(1), // Instructions
            ])
            .split(area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // File browser
                Constraint::Percentage(50), // File details and content
            ])
            .split(chunks[0]);

        self.file_browser.render(frame, main_chunks[0]);

        let details_content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // File details
                Constraint::Min(1),    // File content
            ])
            .split(main_chunks[1]);

        self.render_file_details(frame, details_content_chunks[0]);
        self.render_file_content(frame, details_content_chunks[1]);
        self.render_instructions(frame, chunks[1]);
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: Option<&'b mut ThreadedChatSession>,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError> {
        let modal_action = self.file_browser.handle_key_event(key_event)?;

        match key_event.current_key().code {
            KeyCode::Enter | KeyCode::Up | KeyCode::Down => {
                self.update_selected_file_info().await?;
            }
            KeyCode::Esc => {
                return Ok(WindowEvent::PromptWindow(None));
            }
            _ => {}
        }
        Ok(WindowEvent::Modal(modal_action))
    }

    async fn poll_background_task(
        &mut self,
    ) -> Result<WindowEvent, ApplicationError> {
        self.file_browser.poll_background_task().await?;
        self.update_selected_file_info().await?;
        Ok(WindowEvent::Modal(ModalAction::PollBackGroundTask))
    }
}
