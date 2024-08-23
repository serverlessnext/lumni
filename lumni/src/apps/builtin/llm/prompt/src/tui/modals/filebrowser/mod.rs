use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use lumni::{EnvironmentConfig, ObjectStoreHandler, Table, TableColumnValue};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;
use tokio::sync::mpsc;

use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, ModalAction,
    ModalWindowTrait, ModalWindowType, ThreadedChatSession, WindowEvent,
};
pub use crate::external as lumni;

// TODO notes:
// - search options, simple regex /s, or /w where size < 1 AND name ==
// - ability to pin files, hide/ unhide pins
// - ability to select via spacebar
// - ability to copy one or more file paths to clipboard

enum FileOperation {
    ListFiles(String),
    EnterDirectory(String),
    GoUpDirectory(String),
}

#[derive(Clone)]
struct FileListHandler {
    handler: Arc<ObjectStoreHandler>,
}

impl FileListHandler {
    fn new(handler: Arc<ObjectStoreHandler>) -> Self {
        Self { handler }
    }

    async fn list_files(
        &self,
        path: String,
    ) -> Result<Arc<Box<dyn Table + Send + Sync>>, ApplicationError> {
        let query = format!("SELECT * FROM \"localfs://{}\" LIMIT 100", path);
        let config = EnvironmentConfig::new(HashMap::new());
        self.handler
            .execute_query(&query, &config, true, false, None, None)
            .await
            .map(|table| Arc::new(table as Box<dyn Table + Send + Sync>))
            .map_err(|e| ApplicationError::InternalError(e.to_string()))
    }
}

pub enum BackgroundTaskResult {
    FileList(Result<Arc<Box<dyn Table + Send + Sync>>, ApplicationError>),
    DirectoryChange(Result<String, ApplicationError>),
}

pub struct FileBrowserModal {
    current_path: String,
    file_table: Option<Arc<Box<dyn Table + Send + Sync>>>,
    selected_index: usize,
    scroll_offset: usize,
    filter: Option<String>,
    background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    operation_sender: mpsc::Sender<FileOperation>,
    task_start_time: Option<Instant>,
}

impl FileBrowserModal {
    pub fn new(initial_path: String) -> Self {
        let (op_tx, op_rx) = mpsc::channel(100);
        let (result_tx, result_rx) = mpsc::channel(100);

        let handler = Arc::new(ObjectStoreHandler::new(None));

        thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                Self::background_task(op_rx, result_tx, handler).await;
            });
        });

        let mut modal = Self {
            current_path: initial_path,
            file_table: None,
            selected_index: 0,
            scroll_offset: 0,
            filter: None,
            background_task: Some(result_rx),
            operation_sender: op_tx,
            task_start_time: None,
        };

        modal.start_list_files();

        modal
    }

    fn reset_selection(&mut self) {
        self.selected_index = 0;
    }

    fn start_list_files(&mut self) {
        let _ = self
            .operation_sender
            .try_send(FileOperation::ListFiles(self.current_path.clone()));
        self.task_start_time = Some(Instant::now());
    }

    fn start_enter_directory(&mut self) {
        if let Some(table) = &self.file_table {
            if let Some(row) = table.get_row(self.selected_index) {
                if let Some(TableColumnValue::StringColumn(name)) =
                    row.get_value("name")
                {
                    let is_dir = name.ends_with('/');
                    if is_dir {
                        let new_path = if self.current_path == "." {
                            name.to_string()
                        } else {
                            let mut path_buf =
                                PathBuf::from(&self.current_path);
                            path_buf.push(name.trim_end_matches('/'));
                            path_buf.to_string_lossy().into_owned()
                        };
                        let _ = self
                            .operation_sender
                            .try_send(FileOperation::EnterDirectory(new_path));
                        self.task_start_time = Some(Instant::now());
                    } else {
                        // Handle file selection (e.g., open file, show details, etc.)
                        log::debug!("TODO: Handle file selection");
                    }
                }
            }
        }
    }

    fn start_go_up_directory(&mut self) {
        let _ = self
            .operation_sender
            .try_send(FileOperation::GoUpDirectory(self.current_path.clone()));
        self.task_start_time = Some(Instant::now());
    }

    fn render_file_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = if let Some(table) = &self.file_table {
            (0..table.len())
                .filter_map(|i| {
                    if let Some(row) = table.get_row(i) {
                        let full_name = row
                            .get_value("name")
                            .and_then(|v| match v {
                                TableColumnValue::StringColumn(s) => {
                                    Some(s.clone())
                                }
                                _ => None,
                            })
                            .unwrap_or_default();

                        let is_dir = full_name.ends_with('/');
                        let basename = Path::new(&full_name)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&full_name);

                        let icon = if is_dir { "📁 " } else { "📄 " };
                        Some(ListItem::new(Span::raw(format!(
                            "{}{}",
                            icon, basename
                        ))))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        };

        let list_height = area.height as usize - 2; // Subtract 2 for the borders
        let total_items = items.len();

        // Calculate the maximum scroll offset
        let max_scroll = total_items.saturating_sub(list_height);

        // Adjust scroll_offset if necessary
        if self.selected_index >= self.scroll_offset + list_height {
            self.scroll_offset =
                (self.selected_index - list_height + 1).min(max_scroll);
        } else if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }

        // Ensure scroll_offset doesn't exceed max_scroll
        self.scroll_offset = self.scroll_offset.min(max_scroll);

        let items = items
            .into_iter()
            .skip(self.scroll_offset)
            .take(list_height)
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::default().title("Files").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index - self.scroll_offset));

        frame.render_stateful_widget(list, area, &mut list_state);

        if total_items > list_height {
            self.render_scrollbar(frame, area, total_items, list_height);
        }
    }

    fn render_scrollbar(
        &self,
        frame: &mut Frame,
        area: Rect,
        total_items: usize,
        list_height: usize,
    ) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        let max_scroll = total_items.saturating_sub(list_height);
        let scroll_position = (self.scroll_offset as f64 / max_scroll as f64
            * (list_height - 1) as f64)
            .round() as usize;

        frame.render_stateful_widget(
            scrollbar,
            scrollbar_area,
            &mut ScrollbarState::new(list_height)
                .position(scroll_position.min(list_height - 1)),
        );
    }

    async fn background_task(
        mut op_rx: mpsc::Receiver<FileOperation>,
        result_tx: mpsc::Sender<BackgroundTaskResult>,
        handler: Arc<ObjectStoreHandler>,
    ) {
        let file_list_handler = FileListHandler::new(handler.clone());

        while let Some(op) = op_rx.recv().await {
            match op {
                FileOperation::ListFiles(path) => {
                    let result = file_list_handler.list_files(path).await;
                    let _ = result_tx
                        .send(BackgroundTaskResult::FileList(result))
                        .await;
                }
                FileOperation::EnterDirectory(path) => {
                    let new_path = if path == "." {
                        path
                    } else {
                        Path::new(&path).to_string_lossy().into_owned()
                    };

                    let query = if new_path == "." {
                        "SELECT * FROM \"localfs://\" LIMIT 1".to_string()
                    } else {
                        format!(
                            "SELECT * FROM \"localfs://{}/\" LIMIT 1",
                            new_path.trim_end_matches('/')
                        )
                    };

                    let config = EnvironmentConfig::new(HashMap::new());
                    let result = handler
                        .execute_query(&query, &config, true, false, None, None)
                        .await
                        .map_err(|e| {
                            ApplicationError::InternalError(e.to_string())
                        });

                    let directory_change_result = match result {
                        Ok(_) => Ok(new_path),
                        Err(e) => Err(ApplicationError::InvalidInput(format!(
                            "Failed to enter directory: {}",
                            e
                        ))),
                    };

                    let _ = result_tx
                        .send(BackgroundTaskResult::DirectoryChange(
                            directory_change_result,
                        ))
                        .await;
                }
                FileOperation::GoUpDirectory(path) => {
                    let result = if path == "." {
                        Ok(".".to_string()) // Already at root, stay there
                    } else {
                        Path::new(&path)
                            .parent()
                            .map(|p| {
                                if p.as_os_str().is_empty() {
                                    ".".to_string()
                                } else {
                                    p.to_string_lossy().into_owned()
                                }
                            })
                            .ok_or_else(|| {
                                ApplicationError::InvalidInput(
                                    "Cannot go up from root directory"
                                        .to_string(),
                                )
                            })
                    };

                    let _ = result_tx
                        .send(BackgroundTaskResult::DirectoryChange(result))
                        .await;
                }
            }
        }
    }

    async fn handle_background_task_result(
        &mut self,
        result: BackgroundTaskResult,
    ) -> Result<(), ApplicationError> {
        match result {
            BackgroundTaskResult::FileList(result) => {
                self.task_start_time = None;
                match result {
                    Ok(table) => {
                        self.file_table = Some(table);
                        self.apply_filter();
                        self.reset_selection(); // Reset selection when new file list is loaded
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            BackgroundTaskResult::DirectoryChange(result) => {
                self.task_start_time = None;
                match result {
                    Ok(new_path) => {
                        self.current_path = new_path;
                        self.start_list_files();
                        self.reset_selection(); // Reset selection when changing directory
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn apply_filter(&mut self) {
        if let Some(_filter) = &self.filter {
            // Implement filter logic here
        }
    }

    fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        if let Some(table) = &self.file_table {
            if self.selected_index < table.len() - 1 {
                self.selected_index += 1;
            }
        }
    }

    fn page_up(&mut self) {
        let list_height = 10;
        if self.selected_index > list_height {
            self.selected_index -= list_height;
        } else {
            self.selected_index = 0;
        }
        self.scroll_offset = self.scroll_offset.saturating_sub(list_height);
    }

    fn page_down(&mut self) {
        let list_height = 10; // Adjust this value based on your actual list height
        if let Some(table) = &self.file_table {
            let max_index = table.len() - 1;
            if self.selected_index + list_height < max_index {
                self.selected_index += list_height;
            } else {
                self.selected_index = max_index;
            }
            let max_scroll = table.len().saturating_sub(list_height);
            self.scroll_offset =
                (self.scroll_offset + list_height).min(max_scroll);
        }
    }

    fn render_current_path(&self, frame: &mut Frame, area: Rect) {
        let path = Paragraph::new(Span::raw(&self.current_path)).block(
            Block::default().title("Current Path").borders(Borders::ALL),
        );
        frame.render_widget(path, area);
    }

    fn render_file_details(&self, frame: &mut Frame, area: Rect) {
        if let Some(table) = &self.file_table {
            if let Some(row) = table.get_row(self.selected_index) {
                let name = row
                    .get_value("name")
                    .and_then(|v| match v {
                        TableColumnValue::StringColumn(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let is_dir = name.ends_with('/');
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

                let details = vec![
                    Line::from(Span::raw(format!("Name: {}", name))),
                    Line::from(Span::raw(format!(
                        "Type: {}",
                        if is_dir { "Directory" } else { "File" }
                    ))),
                    Line::from(Span::raw(format!("Size: {} bytes", size))),
                    Line::from(Span::raw(format!("Modified: {}", modified))), // You might want to format this timestamp
                ];

                let paragraph = Paragraph::new(details).block(
                    Block::default()
                        .title("File Details")
                        .borders(Borders::ALL),
                );
                frame.render_widget(paragraph, area);
            }
        }
    }

    fn render_instructions(&self, frame: &mut Frame, area: Rect) {
        let instructions = "↑↓: Navigate | Enter: Open Directory | Backspace: \
                            Go Up | F: Filter | Esc: Close";
        let paragraph = Paragraph::new(Span::raw(instructions))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(paragraph, area);
    }

    fn render_loading(&self, frame: &mut Frame, area: Rect) {
        if self.task_start_time.is_some() {
            let elapsed = self
                .task_start_time
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0);
            let message = format!("Loading... ({} seconds)", elapsed);
            let loading = Paragraph::new(Span::raw(message))
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(loading, area);
        }
    }
}

#[async_trait]
impl ModalWindowTrait for FileBrowserModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::FileBrowser
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Current path
                Constraint::Min(1),    // File list
                Constraint::Length(5), // File details
                Constraint::Length(1), // Instructions
            ])
            .split(area);

        self.render_current_path(frame, chunks[0]);
        self.render_file_list(frame, chunks[1]);
        self.render_file_details(frame, chunks[2]);
        self.render_instructions(frame, chunks[3]);

        if self.task_start_time.is_some() {
            self.render_loading(frame, chunks[1]);
        }
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: &'b mut ThreadedChatSession,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Up => self.move_selection_up(),
            KeyCode::Down => self.move_selection_down(),
            KeyCode::Enter => {
                self.start_enter_directory();
            }
            KeyCode::PageUp | KeyCode::Char('k') => self.page_up(),
            KeyCode::PageDown | KeyCode::Char('j') => self.page_down(),
            KeyCode::Backspace => {
                self.start_go_up_directory();
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // TODO: search files
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                return Ok(WindowEvent::PromptWindow(None))
            }
            _ => {}
        }
        Ok(WindowEvent::Modal(ModalAction::Refresh))
    }

    async fn refresh(&mut self) -> Result<WindowEvent, ApplicationError> {
        if let Some(ref mut rx) = self.background_task {
            match rx.try_recv() {
                Ok(result) => {
                    self.handle_background_task_result(result).await?;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(ApplicationError::InternalError(
                        "Background task disconnected".to_string(),
                    ));
                }
            }
        }
        Ok(WindowEvent::Modal(ModalAction::Refresh))
    }
}
