use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossterm::event::KeyCode;
use dirs::home_dir;
use lumni::api::error::ApplicationError;
use lumni::{
    EnvironmentConfig, FileType, ObjectStoreHandler, Table, TableColumnValue,
    TableRow,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph,
    Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;
use tokio::sync::mpsc;

use super::{
    KeyTrack, ModalAction, TextArea, TextWindowTrait,
};
pub use crate::external as lumni;

// TODO notes:
// - search options, simple regex /s, or /w where size < 1 AND name ==
// - ability to pin files, hide/ unhide pins
// - ability to select via spacebar
// - ability to copy one or more file paths to clipboard

enum FileOperation {
    ListFiles(String, Option<String>), // (path, file_to_select)
}

#[derive(Clone)]
struct FileListHandler {
    handler: Arc<ObjectStoreHandler>,
}

impl FileListHandler {
    fn new(handler: Arc<ObjectStoreHandler>) -> Self {
        Self { handler }
    }

    async fn handle_query(
        &self,
        query: String,
    ) -> Result<Arc<Box<dyn Table + Send + Sync>>, ApplicationError> {
        let config = EnvironmentConfig::new(HashMap::new());
        let skip_hidden = true;
        let recursive = false;
        let result = self
            .handler
            .execute_query(&query, &config, skip_hidden, recursive, None, None)
            .await
            .map_err(ApplicationError::from)?;
        Ok(Arc::new(result as Box<dyn Table + Send + Sync>))
    }

    async fn list_files(
        &self,
        path: String,
    ) -> Result<Arc<Box<dyn Table + Send + Sync>>, ApplicationError> {
        let query = format!("SELECT * FROM \"localfs://{}\" LIMIT 100", path);
        match self.handle_query(query).await {
            Ok(table) => Ok(table),
            Err(ApplicationError::NotFound(_)) => {
                let query =
                    "SELECT * FROM \"localfs://.\" LIMIT 100".to_string();
                self.handle_query(query).await
            }
            Err(e) => match e {
                _ => Err(ApplicationError::InternalError(e.to_string())),
            },
        }
    }
}

pub enum BackgroundTaskResult {
    FileList(
        Result<Arc<Box<dyn Table + Send + Sync>>, ApplicationError>,
        Option<String>,
    ),
}

pub struct FileBrowserWidget<'a> {
    base_path: PathBuf,
    current_path: PathBuf,
    path_input: TextArea<'a>,
    file_table: Option<Arc<Box<dyn Table + Send + Sync>>>,
    selected_index: usize,
    displayed_index: usize,
    scroll_offset: usize,
    background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    operation_sender: mpsc::Sender<FileOperation>,
    task_start_time: Option<Instant>,
    focus: FileBrowserFocus,
    filter_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileBrowserFocus {
    PathInput,
    FileList,
}

impl<'a> FileBrowserWidget<'a> {
    pub fn new(base_path: Option<PathBuf>) -> Self {
        let base_path = base_path
            .or_else(home_dir)
            .unwrap_or_else(|| PathBuf::from("/"));
        let current_path = base_path.clone();

        let (op_tx, op_rx) = mpsc::channel(100);
        let (result_tx, result_rx) = mpsc::channel(100);

        let handler = Arc::new(ObjectStoreHandler::new(None));

        thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                Self::background_task(op_rx, result_tx, handler).await;
            });
        });

        let mut path_input = TextArea::new();
        path_input.text_set("", None).unwrap(); // Initialize with empty string

        let mut widget = Self {
            base_path,
            current_path,
            path_input,
            file_table: None,
            selected_index: 0,
            displayed_index: 0,
            scroll_offset: 0,
            background_task: Some(result_rx),
            operation_sender: op_tx,
            task_start_time: None,
            focus: FileBrowserFocus::FileList,
            filter_text: String::new(),
        };

        widget.start_list_files();

        widget
    }

    pub fn get_selected_table_row(&self) -> Option<TableRow> {
        if let Some(table) = &self.file_table {
            if let Some(row) = table.get_row(self.selected_index) {
                return Some(row);
            }
        }
        None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if area.height < 8 {
            let message =
                Paragraph::new("Not enough space to display file list")
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
            frame.render_widget(message, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Visual path display
                Constraint::Length(3), // Editable path input
                Constraint::Min(1),    // File list
            ])
            .split(area);

        self.render_visual_path(frame, chunks[0]);
        self.render_path_input(frame, chunks[1]);
        self.render_file_list(frame, chunks[2]);

        if self.task_start_time.is_some() {
            self.render_loading(frame, area);
        }
    }

    fn render_visual_path(&self, frame: &mut Frame, area: Rect) {
        let relative_path = self
            .current_path
            .strip_prefix(&self.base_path)
            .unwrap_or(&self.current_path);
        let path_parts: Vec<&str> = relative_path
            .to_str()
            .unwrap_or("")
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut path_spans = vec![Span::styled(
            self.base_path.to_str().unwrap_or(""),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )];

        if !path_parts.is_empty() {
            path_spans.push(Span::raw(" → "));
            for (i, part) in path_parts.iter().enumerate() {
                path_spans.push(Span::styled(
                    *part,
                    Style::default().fg(Color::Yellow),
                ));
                if i < path_parts.len() - 1 {
                    path_spans.push(Span::raw(" / "));
                }
            }
        }

        let path_widget = Paragraph::new(Line::from(path_spans))
            .block(Block::default().borders(Borders::ALL).title("Path"))
            .alignment(Alignment::Left);

        frame.render_widget(path_widget, area);
    }

    fn render_path_input(&mut self, frame: &mut Frame, area: Rect) {
        let (input_style, border_style) = match self.focus {
            FileBrowserFocus::PathInput => (
                Style::default().fg(Color::Yellow),
                Style::default().fg(Color::Yellow),
            ),
            FileBrowserFocus::FileList => (Style::default(), Style::default()),
        };

        let input_widget = self.path_input.widget(&area).style(input_style);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title("Enter file/dir name");

        frame.render_widget(block, area);
        frame.render_widget(
            input_widget,
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
        );
    }

    fn update_selection(&mut self) {
        if self.filter_text.is_empty() {
            self.displayed_index = usize::MAX; // No selection
            self.selected_index = usize::MAX; // No selection
        } else {
            self.displayed_index = 0;
            self.scroll_offset = 0;
        }
    }

    fn has_selection(&self) -> bool {
        self.displayed_index != usize::MAX
    }

    fn render_file_list(&mut self, frame: &mut Frame, area: Rect) {
        let mut filtered_indices = Vec::new();
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

                        let basename = Path::new(&full_name)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&full_name)
                            .to_string();

                        // Apply filter to both directories and files using basename
                        if !self.filter_text.is_empty() {
                            let lowercase_basename = basename.to_lowercase();
                            let lowercase_filter =
                                self.filter_text.to_lowercase();
                            if !lowercase_basename
                                .starts_with(&lowercase_filter)
                            {
                                return None;
                            }
                        }

                        filtered_indices.push(i);

                        let file_type = match row.get_value("type") {
                            Some(TableColumnValue::Uint8Column(value)) => {
                                FileType::from_u8(*value)
                            }
                            _ => FileType::Unknown,
                        };
                        let is_dir = file_type == FileType::Directory;

                        let (icon, style) = if is_dir {
                            (
                                "📁 ".to_string(),
                                Style::default().fg(Color::Cyan),
                            )
                        } else {
                            (
                                "📄 ".to_string(),
                                Style::default().fg(Color::White),
                            )
                        };

                        Some(ListItem::new(Line::from(vec![
                            Span::styled(icon, style),
                            Span::styled(basename, style),
                        ])))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        };

        // Update the actual selected index based on the displayed index
        if !filtered_indices.is_empty() && self.has_selection() {
            self.selected_index = filtered_indices
                [self.displayed_index.min(filtered_indices.len() - 1)];
        } else {
            self.selected_index = usize::MAX;
        }

        let list_height = area.height.saturating_sub(2) as usize;
        let total_items = items.len();

        // Adjust scroll_offset if necessary
        if self.has_selection() {
            if self.displayed_index >= self.scroll_offset + list_height {
                self.scroll_offset =
                    self.displayed_index.saturating_sub(list_height) + 1;
            } else if self.displayed_index < self.scroll_offset {
                self.scroll_offset = self.displayed_index;
            }
        } else {
            // Reset scroll offset when there's no selection
            self.scroll_offset = 0;
        }

        // Ensure scroll_offset doesn't exceed max_scroll
        let max_scroll = total_items.saturating_sub(list_height);
        self.scroll_offset = self.scroll_offset.min(max_scroll);

        let items = items
            .into_iter()
            .skip(self.scroll_offset)
            .take(list_height)
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::default().title("Files").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        let mut list_state = ListState::default();
        if self.focus == FileBrowserFocus::FileList && self.has_selection() {
            list_state.select(Some(
                self.displayed_index.saturating_sub(self.scroll_offset),
            ));
        } else {
            list_state.select(None);
        }

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
            * (list_height.saturating_sub(1)) as f64)
            .round() as usize;

        frame.render_stateful_widget(
            scrollbar,
            scrollbar_area,
            &mut ScrollbarState::new(list_height)
                .position(scroll_position.min(list_height.saturating_sub(1))),
        );
    }

    fn render_loading(&self, frame: &mut Frame, area: Rect) {
        if let Some(start_time) = self.task_start_time {
            let elapsed = start_time.elapsed().as_secs();
            let message = format!("Loading... ({} seconds)", elapsed);
            let loading = Paragraph::new(Span::raw(message))
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(loading, area);
        }
    }

    fn submit_path_input(&mut self) -> Result<(), ApplicationError> {
        let input = self.path_input.text_buffer().to_string();
        if !input.is_empty() {
            let new_path = self.current_path.join(input);
            if new_path.exists() {
                self.current_path = new_path;
                self.start_list_files();
            } else {
                // TODO: Handle non-existent path (e.g., show an error message)
            }
        }
        Ok(())
    }

    pub fn handle_key_event(
        &mut self,
        key_event: &mut KeyTrack,
    ) -> Result<ModalAction, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Char(c) => {
                self.focus = FileBrowserFocus::PathInput;
                self.path_input.set_status_insert();
                self.path_input.process_edit_input(key_event)?;
                self.filter_text.push(c);
                self.update_selection();
            }
            KeyCode::Backspace => {
                if !self.filter_text.is_empty() {
                    self.filter_text.pop();
                    self.path_input.process_edit_input(key_event)?;
                    self.update_selection();
                } else {
                    self.go_up_directory();
                }
            }
            KeyCode::Down => {
                if self.focus == FileBrowserFocus::PathInput {
                    self.focus = FileBrowserFocus::FileList;
                    if !self.has_selection() {
                        self.displayed_index = 0;
                    }
                } else {
                    self.move_selection_down();
                }
            }
            KeyCode::Up => {
                if self.focus == FileBrowserFocus::FileList {
                    if self.displayed_index == 0 {
                        self.focus = FileBrowserFocus::PathInput;
                        self.displayed_index = usize::MAX;
                    } else {
                        self.move_selection_up();
                    }
                }
            }
            KeyCode::Tab => {
                self.enter_directory();
                self.focus = FileBrowserFocus::FileList;
            }
            KeyCode::Enter => {
                if self.focus == FileBrowserFocus::PathInput {
                    self.submit_path_input()?;
                    self.focus = FileBrowserFocus::FileList;
                    self.clear_path_input()?;
                } else {
                    self.enter_directory();
                }
            }
            KeyCode::PageUp => {
                if self.focus == FileBrowserFocus::FileList {
                    self.page_up();
                }
            }
            KeyCode::PageDown => {
                if self.focus == FileBrowserFocus::FileList {
                    self.page_down();
                } else if self.focus == FileBrowserFocus::PathInput {
                    // If in path input, behave like Down key
                    self.focus = FileBrowserFocus::FileList;
                    if !self.has_selection() {
                        self.displayed_index = 0;
                        self.update_selected_index();
                    }
                }
            }
            KeyCode::Esc => {
                self.focus = FileBrowserFocus::FileList;
                self.clear_path_input()?;
            }
            _ => {}
        }
        Ok(ModalAction::WaitForKeyEvent)
    }

    fn page_up(&mut self) {
        let list_height = 10;
        if self.displayed_index == 0 {
            // If at the top, move focus to path input
            self.focus = FileBrowserFocus::PathInput;
            self.displayed_index = usize::MAX; // Indicate no selection
        } else if self.displayed_index > list_height {
            self.displayed_index -= list_height;
        } else {
            self.displayed_index = 0;
        }
        self.scroll_offset = self.scroll_offset.saturating_sub(list_height);
        self.update_selected_index();
    }

    fn page_down(&mut self) {
        let list_height = 10;
        if let Some(table) = &self.file_table {
            let filtered_count = self.get_filtered_count(table);
            let max_index = filtered_count.saturating_sub(1);
            if self.displayed_index + list_height < max_index {
                self.displayed_index += list_height;
            } else {
                self.displayed_index = max_index;
            }
            let max_scroll = filtered_count.saturating_sub(list_height);
            self.scroll_offset =
                (self.scroll_offset + list_height).min(max_scroll);
            self.update_selected_index();
        }
    }

    fn get_filtered_count(
        &self,
        table: &Arc<Box<dyn Table + Send + Sync>>,
    ) -> usize {
        (0..table.len())
            .filter(|&i| {
                if let Some(row) = table.get_row(i) {
                    if let Some(TableColumnValue::StringColumn(name)) =
                        row.get_value("name")
                    {
                        let basename = Path::new(name)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(name);
                        basename
                            .to_lowercase()
                            .starts_with(&self.filter_text.to_lowercase())
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .count()
    }

    fn update_selected_index(&mut self) {
        if let Some(table) = &self.file_table {
            let filtered_indices: Vec<usize> = (0..table.len())
                .filter(|&i| {
                    if let Some(row) = table.get_row(i) {
                        if let Some(TableColumnValue::StringColumn(name)) =
                            row.get_value("name")
                        {
                            let basename = Path::new(name)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(name);
                            basename
                                .to_lowercase()
                                .starts_with(&self.filter_text.to_lowercase())
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .collect();

            if !filtered_indices.is_empty() {
                self.selected_index = filtered_indices
                    [self.displayed_index.min(filtered_indices.len() - 1)];
            } else {
                self.selected_index = usize::MAX;
            }
        }
    }

    fn get_selected_path(&self) -> Option<PathBuf> {
        if let Some(table) = &self.file_table {
            if let Some(row) = table.get_row(self.selected_index) {
                if let Some(TableColumnValue::StringColumn(name)) =
                    row.get_value("name")
                {
                    return Some(
                        self.current_path.join(name.trim_end_matches('/')),
                    );
                }
            }
        }
        None
    }

    fn start_list_files(&mut self) {
        let _ = self.operation_sender.try_send(FileOperation::ListFiles(
            self.current_path.to_string_lossy().into_owned(),
            None,
        ));
        self.task_start_time = Some(Instant::now());
        self.filter_text.clear();
    }

    fn enter_directory(&mut self) {
        if let Some(path) = self.get_selected_path() {
            if path.is_dir() {
                self.current_path = path;
                self.start_list_files();
                self.clear_path_input().unwrap_or_default();
            }
        }
    }

    fn clear_path_input(&mut self) -> Result<(), ApplicationError> {
        self.path_input.text_set("", None)?;
        self.filter_text.clear();
        Ok(())
    }

    fn go_up_directory(&mut self) {
        if self.current_path != self.base_path {
            let dir_to_select = self
                .current_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|s| s.to_string());
            self.current_path.pop();
            let _ = self.operation_sender.try_send(FileOperation::ListFiles(
                self.current_path.to_string_lossy().into_owned(),
                dir_to_select,
            ));
            self.task_start_time = Some(Instant::now());
            self.clear_path_input().unwrap_or_default();
            self.filter_text.clear();
            self.displayed_index = 0;
            self.scroll_offset = 0;
        }
    }

    fn move_selection_up(&mut self) {
        if self.has_selection() && self.displayed_index > 0 {
            self.displayed_index -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        if let Some(table) = &self.file_table {
            let filtered_count = (0..table.len())
                .filter(|&i| {
                    if let Some(row) = table.get_row(i) {
                        if let Some(TableColumnValue::StringColumn(name)) =
                            row.get_value("name")
                        {
                            let basename = Path::new(name)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(name);
                            basename
                                .to_lowercase()
                                .starts_with(&self.filter_text.to_lowercase())
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .count();
            let max_index = filtered_count.saturating_sub(1);
            if self.has_selection() && self.displayed_index < max_index {
                self.displayed_index += 1;
            } else if !self.has_selection() && filtered_count > 0 {
                self.displayed_index = 0;
            }
        }
    }

    pub async fn refresh(&mut self) -> Result<(), ApplicationError> {
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
        Ok(())
    }

    async fn handle_background_task_result(
        &mut self,
        result: BackgroundTaskResult,
    ) -> Result<(), ApplicationError> {
        match result {
            BackgroundTaskResult::FileList(result, file_to_select) => {
                self.task_start_time = None;
                match result {
                    Ok(table) => {
                        self.file_table = Some(table);
                        if let Some(name) = file_to_select {
                            self.select_file_by_name(&name);
                        } else {
                            self.selected_index = 0;
                            self.displayed_index = 0;
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn select_file_by_name(&mut self, name: &str) {
        if let Some(table) = &self.file_table {
            let normalized_name = name.trim_end_matches('/');
            for (index, row) in (0..table.len())
                .filter_map(|i| table.get_row(i))
                .enumerate()
            {
                if let Some(TableColumnValue::StringColumn(file_name)) =
                    row.get_value("name")
                {
                    let normalized_file_name = file_name.trim_end_matches('/');
                    if normalized_file_name == normalized_name {
                        self.selected_index = index;
                        self.displayed_index = index;
                        break;
                    }
                }
            }
        }
    }

    async fn background_task(
        mut op_rx: mpsc::Receiver<FileOperation>,
        result_tx: mpsc::Sender<BackgroundTaskResult>,
        handler: Arc<ObjectStoreHandler>,
    ) {
        let file_list_handler = FileListHandler::new(handler.clone());

        while let Some(op) = op_rx.recv().await {
            match op {
                FileOperation::ListFiles(path, file_to_select) => {
                    let result = file_list_handler.list_files(path).await;
                    let _ = result_tx
                        .send(BackgroundTaskResult::FileList(
                            result,
                            file_to_select,
                        ))
                        .await;
                }
            }
        }
    }
}
