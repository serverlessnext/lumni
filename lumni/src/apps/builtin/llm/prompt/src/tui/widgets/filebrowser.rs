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
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, StatefulWidget, StatefulWidgetRef, Widget, Wrap,
};
use tokio::sync::mpsc;

use super::list::{ListWidget, ListWidgetState};
use super::{KeyTrack, ModalEvent, PromptWindow, TextWindowTrait};
pub use crate::external as lumni;

// TODO notes:
// - search options, simple regex /s, or /w where size < 1 AND name ==
// - ability to pin files, hide/ unhide pins
// - ability to select via spacebar
// - ability to copy one or more file paths to clipboard

const PAGE_SIZE: usize = 10;

#[derive(Debug)]
pub struct FileBrowser {
    pub widget: FileBrowserWidget,
    pub state: FileBrowserState<'static>,
}

impl FileBrowser {
    pub fn new(base_path: Option<PathBuf>) -> Self {
        let (widget, state) = FileBrowserWidget::new(base_path);
        Self { widget, state }
    }

    pub fn handle_key_event(
        &mut self,
        key_event: &mut KeyTrack,
    ) -> Result<ModalEvent, ApplicationError> {
        self.widget.handle_key_event(key_event, &mut self.state)
    }

    pub async fn poll_background_task(
        &mut self,
    ) -> Result<Option<ModalEvent>, ApplicationError> {
        self.widget.poll_background_task(&mut self.state).await
    }

    pub fn current_path(&self) -> &Path {
        &self.widget.current_path
    }

    pub fn get_selected_file(&self) -> Option<PathBuf> {
        self.widget.get_selected_path(&self.state)
    }
}

enum FileOperation {
    ListFiles(String, Option<String>), // (path, file_to_select)
}

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub enum BackgroundTaskResult {
    FileList(
        Result<Arc<Box<dyn Table + Send + Sync>>, ApplicationError>,
        Option<String>,
    ),
}

#[derive(Debug)]
pub struct FileBrowserState<'a> {
    path_input: PromptWindow<'a>,
    focus: FileBrowserFocus,
    filter_text: String,
    task_start_time: Option<Instant>,
    list_state: ListWidgetState,
}

impl<'a> Default for FileBrowserState<'a> {
    fn default() -> Self {
        let mut path_input = PromptWindow::new();
        path_input.text_set("", None).unwrap();
        Self {
            path_input,
            focus: FileBrowserFocus::FileList,
            filter_text: String::new(),
            task_start_time: None,
            list_state: ListWidgetState::default(),
        }
    }
}
#[derive(Debug)]
pub struct FileBrowserWidget {
    base_path: PathBuf,
    current_path: PathBuf,
    file_table: Option<Arc<Box<dyn Table + Send + Sync>>>,
    background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    operation_sender: mpsc::Sender<FileOperation>,
    list_widget: ListWidget,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileBrowserFocus {
    PathInput,
    FileList,
}

impl FileBrowserWidget {
    pub fn new(
        base_path: Option<PathBuf>,
    ) -> (Self, FileBrowserState<'static>) {
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

        let mut path_input = PromptWindow::new();
        path_input.text_set("", None).unwrap();

        let list_widget = ListWidget::new(Vec::new())
            .title("Files")
            .normal_style(Style::default().fg(Color::White))
            .selected_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ".to_string())
            .show_borders(false);

        let mut widget = Self {
            base_path,
            current_path,
            file_table: None,
            background_task: Some(result_rx),
            operation_sender: op_tx,
            list_widget,
        };

        let mut state = FileBrowserState::default();

        widget.start_list_files(&mut state);

        (widget, state)
    }

    fn update_list_items(&mut self, state: &mut FileBrowserState) {
        if let Some(table) = &self.file_table {
            let items: Vec<Text<'static>> = (0..table.len())
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

                        if !state.filter_text.is_empty() {
                            let lowercase_basename = basename.to_lowercase();
                            let lowercase_filter =
                                state.filter_text.to_lowercase();
                            if !lowercase_basename
                                .starts_with(&lowercase_filter)
                            {
                                return None;
                            }
                        }

                        let file_type = match row.get_value("type") {
                            Some(TableColumnValue::Uint8Column(value)) => {
                                FileType::from_u8(*value)
                            }
                            _ => FileType::Unknown,
                        };
                        let is_dir = file_type == FileType::Directory;

                        let icon = if is_dir { "📁 " } else { "📄 " };
                        let style = if is_dir {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        Some(Text::from(Line::from(Span::styled(
                            format!("{}{}", icon, basename),
                            style,
                        ))))
                    } else {
                        None
                    }
                })
                .collect();

            self.list_widget = ListWidget::new(items)
                .title("Files")
                .normal_style(Style::default().fg(Color::White))
                .selected_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("► ".to_string())
                .show_borders(false);
        }
    }

    fn get_selected_path(&self, state: &FileBrowserState) -> Option<PathBuf> {
        self.list_widget
            .get_selected_item(&state.list_state)
            .and_then(|item| {
                item.lines.first().and_then(|line| {
                    line.spans.first().map(|span| {
                        let name =
                            span.content.trim_start_matches(|c: char| {
                                c.is_whitespace() || c == '📁' || c == '📄'
                            });
                        self.current_path.join(name.trim_end_matches('/'))
                    })
                })
            })
    }

    pub fn get_selected_table_row(
        &self,
        state: &FileBrowserState,
    ) -> Option<TableRow> {
        if let Some(table) = &self.file_table {
            let selected_item =
                self.list_widget.get_selected_item(&state.list_state)?;
            let selected_name = selected_item
                .lines
                .first()?
                .spans
                .first()?
                .content
                .trim_start_matches(|c: char| {
                    c.is_whitespace() || c == '📁' || c == '📄'
                })
                .trim_end_matches('/');

            for i in 0..table.len() {
                if let Some(row) = table.get_row(i) {
                    if let Some(TableColumnValue::StringColumn(name)) =
                        row.get_value("name")
                    {
                        if name.trim_end_matches('/') == selected_name {
                            return Some(row);
                        }
                    }
                }
            }
        }
        None
    }

    fn render_visual_path(&self, buf: &mut Buffer, area: Rect) {
        let relative_path = self
            .current_path
            .strip_prefix(&self.base_path)
            .unwrap_or(&self.current_path);

        let path_str = relative_path.to_str().unwrap_or("");
        let path_parts: Vec<&str> =
            path_str.split('/').filter(|s| !s.is_empty()).collect();

        let mut path_spans = vec![
            Span::styled("📂 ", Style::default().fg(Color::Cyan)),
            Span::styled(
                self.base_path
                    .file_name()
                    .unwrap_or(self.base_path.as_os_str())
                    .to_str()
                    .unwrap_or(""),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        if !path_parts.is_empty() {
            path_spans.push(Span::raw(" / "));

            if path_parts.len() > 2 {
                // Show first part
                path_spans.push(Span::styled(
                    path_parts[0],
                    Style::default().fg(Color::Gray),
                ));
                path_spans.push(Span::raw(" / .. / "));

                // Show last part
                path_spans.push(Span::styled(
                    path_parts[path_parts.len() - 1],
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                // If 2 or fewer parts, show all
                for (i, part) in path_parts.iter().enumerate() {
                    path_spans.push(Span::styled(
                        *part,
                        if i == path_parts.len() - 1 {
                            Style::default()
                                .fg(Color::Gray)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Gray)
                        },
                    ));
                    if i < path_parts.len() - 1 {
                        path_spans.push(Span::raw(" / "));
                    }
                }
            }
        }

        let path_widget = Paragraph::new(Line::from(path_spans))
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });

        path_widget.render(area, buf);
    }

    fn render_path_input(
        &self,
        buf: &mut Buffer,
        area: Rect,
        state: &mut FileBrowserState,
    ) {
        let (input_style, border_style) = match state.focus {
            FileBrowserFocus::PathInput => (
                Style::default().fg(Color::Cyan),
                Style::default().fg(Color::DarkGray),
            ),
            FileBrowserFocus::FileList => (
                Style::default().fg(Color::Gray),
                Style::default().fg(Color::DarkGray),
            ),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title("Enter file/dir name");

        let inner_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        block.render(area, buf);
        state
            .path_input
            .widget(&inner_area)
            .style(input_style)
            .render(inner_area, buf);
    }

    fn get_display_path(&self) -> String {
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

        let base_dir_name = self
            .base_path
            .file_name()
            .unwrap_or(self.base_path.as_os_str())
            .to_str()
            .unwrap_or("/");

        match path_parts.len() {
            0 => base_dir_name.to_string(),
            1 => format!("{}/{}", base_dir_name, path_parts[0]),
            _ => format!(
                "{}/../{}",
                base_dir_name,
                path_parts.last().unwrap_or(&"")
            ),
        }
    }

    fn render_integrated_path_input(
        &self,
        buf: &mut Buffer,
        area: Rect,
        state: &mut FileBrowserState,
    ) {
        let (input_style, border_style) = match state.focus {
            FileBrowserFocus::PathInput => (
                Style::default().fg(Color::Cyan),
                Style::default().fg(Color::Yellow),
            ),
            FileBrowserFocus::FileList => (
                Style::default().fg(Color::Gray),
                Style::default().fg(Color::DarkGray),
            ),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner_area = block.inner(area);
        block.render(area, buf);

        // Render current directory path in the title area
        let display_path = self.get_display_path();
        let mut title = format!("📂 {}", display_path);
        let max_title_width = area.width.saturating_sub(2) as usize; // Leave space for borders

        // Truncate the title if it's too long
        if title.len() > max_title_width {
            title = title
                .chars()
                .take(max_title_width.saturating_sub(3))
                .collect::<String>()
                + "...";
        }

        let title_width = title.len() as u16;
        let title_area = Rect::new(
            area.x + 1,
            area.y,
            title_width.min(area.width.saturating_sub(2)),
            1,
        );
        let title_paragraph = Paragraph::new(title).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        title_paragraph.render(title_area, buf);

        // Render the input field
        let input_area = Rect {
            x: inner_area.x,
            y: inner_area.y,
            width: inner_area.width,
            height: 1,
        };

        state
            .path_input
            .widget(&input_area)
            .style(input_style)
            .render(input_area, buf);
    }

    fn render_loading(
        &self,
        buf: &mut Buffer,
        area: Rect,
        state: &FileBrowserState,
    ) {
        if let Some(start_time) = state.task_start_time {
            let elapsed = start_time.elapsed().as_secs();
            let message = format!("Loading... ({} seconds)", elapsed);
            let loading = Paragraph::new(Span::raw(message))
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            loading.render(area, buf);
        }
    }

    fn submit_path_input(
        &mut self,
        state: &mut FileBrowserState,
    ) -> Result<(), ApplicationError> {
        let input = state.path_input.text_buffer().to_string();
        if !input.is_empty() {
            let new_path = self.current_path.join(input);
            if new_path.exists() {
                self.current_path = new_path;
                self.start_list_files(state);
            } else {
                // TODO: Handle non-existent path (e.g., show an error message)
            }
        }
        Ok(())
    }

    pub fn handle_key_event(
        &mut self,
        key_event: &mut KeyTrack,
        state: &mut FileBrowserState,
    ) -> Result<ModalEvent, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Char(c) => {
                state.focus = FileBrowserFocus::PathInput;
                state.path_input.set_status_insert();
                state.path_input.process_edit_input(key_event)?;
                state.filter_text.push(c);
                self.update_list_items(state);
            }
            KeyCode::Backspace => {
                if !state.filter_text.is_empty() {
                    state.filter_text.pop();
                    state.path_input.process_edit_input(key_event)?;
                    self.update_list_items(state);
                } else {
                    self.go_up_directory(state);
                }
            }
            KeyCode::Down => self.move_selection_down(state),
            KeyCode::Up => self.move_selection_up(state),
            KeyCode::Tab => {
                self.enter_directory(state);
                state.focus = FileBrowserFocus::FileList;
            }
            KeyCode::Enter => {
                if state.focus == FileBrowserFocus::PathInput {
                    self.submit_path_input(state)?;
                    state.focus = FileBrowserFocus::FileList;
                    self.clear_path_input(state)?;
                } else {
                    self.enter_directory(state);
                }
            }
            KeyCode::PageUp => self.page_up(state),
            KeyCode::PageDown => self.page_down(state),
            KeyCode::Esc => {
                state.focus = FileBrowserFocus::FileList;
                self.clear_path_input(state)?;
            }
            _ => {}
        }
        Ok(ModalEvent::UpdateUI)
    }

    fn page_up(&mut self, state: &mut FileBrowserState) {
        if state.list_state.selected_index == 0 {
            // If at the top, move focus to path input
            state.focus = FileBrowserFocus::PathInput;
        } else {
            self.list_widget.page_up(&mut state.list_state, PAGE_SIZE);
        }
    }

    fn page_down(&mut self, state: &mut FileBrowserState) {
        if state.focus == FileBrowserFocus::PathInput {
            state.focus = FileBrowserFocus::FileList;
            state.list_state.selected_index = 0;
        } else {
            self.list_widget.page_down(&mut state.list_state, PAGE_SIZE);
        }
    }

    fn move_selection_up(&mut self, state: &mut FileBrowserState) {
        if state.list_state.selected_index == 0 {
            // If at the top, move focus to path input
            state.focus = FileBrowserFocus::PathInput;
        } else {
            self.list_widget.move_selection(&mut state.list_state, -1);
        }
    }

    fn move_selection_down(&mut self, state: &mut FileBrowserState) {
        if state.focus == FileBrowserFocus::PathInput {
            state.focus = FileBrowserFocus::FileList;
            state.list_state.selected_index = 0;
        } else {
            self.list_widget.move_selection(&mut state.list_state, 1);
        }
    }

    fn start_list_files(&mut self, state: &mut FileBrowserState) {
        let _ = self.operation_sender.try_send(FileOperation::ListFiles(
            self.current_path.to_string_lossy().into_owned(),
            None,
        ));
        state.task_start_time = Some(Instant::now());
        state.filter_text.clear();
        state.list_state = ListWidgetState::default(); // Reset list state
        self.update_list_items(state);
    }

    fn enter_directory(&mut self, state: &mut FileBrowserState) {
        if let Some(path) = self.get_selected_path(state) {
            if path.is_dir() {
                self.current_path = path;
                self.start_list_files(state);
                self.clear_path_input(state).unwrap_or_default();
            }
        }
    }

    fn clear_path_input(
        &mut self,
        state: &mut FileBrowserState,
    ) -> Result<(), ApplicationError> {
        state.path_input.text_set("", None)?;
        state.filter_text.clear();
        Ok(())
    }

    fn go_up_directory(&mut self, state: &mut FileBrowserState) {
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
            state.task_start_time = Some(Instant::now());
            self.clear_path_input(state).unwrap_or_default();
            state.filter_text.clear();
            state.list_state = ListWidgetState::default(); // Reset list state
            self.update_list_items(state);
        }
    }

    pub async fn poll_background_task(
        &mut self,
        state: &mut FileBrowserState<'static>,
    ) -> Result<Option<ModalEvent>, ApplicationError> {
        if let Some(ref mut rx) = self.background_task {
            match rx.try_recv() {
                Ok(result) => {
                    self.handle_background_task_result(result, state).await?;
                    return Ok(Some(ModalEvent::UpdateUI));
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(ApplicationError::InternalError(
                        "Background task disconnected".to_string(),
                    ));
                }
            }
        }
        Ok(None)
    }

    async fn handle_background_task_result(
        &mut self,
        result: BackgroundTaskResult,
        state: &mut FileBrowserState<'static>,
    ) -> Result<(), ApplicationError> {
        match result {
            BackgroundTaskResult::FileList(result, file_to_select) => {
                state.task_start_time = None;
                match result {
                    Ok(table) => {
                        self.file_table = Some(table);
                        self.update_list_items(state);
                        if let Some(name) = file_to_select {
                            self.select_file_by_name(&name, state);
                        } else {
                            state.list_state.selected_index = 0;
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn select_file_by_name(
        &mut self,
        name: &str,
        state: &mut FileBrowserState,
    ) {
        let normalized_name = name.trim_end_matches('/');
        if let Some(index) = self.list_widget.items.iter().position(|item| {
            if let Some(first_line) = item.lines.first() {
                if let Some(first_span) = first_line.spans.first() {
                    let item_name =
                        first_span.content.trim_start_matches(|c: char| {
                            c.is_whitespace() || c == '📁' || c == '📄'
                        });
                    return item_name.trim_end_matches('/') == normalized_name;
                }
            }
            false
        }) {
            state.list_state.selected_index = index;
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

impl StatefulWidget for &FileBrowserWidget {
    type State = FileBrowserState<'static>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidgetRef::render_ref(&self, area, buf, state)
    }
}

impl StatefulWidgetRef for &FileBrowserWidget {
    type State = FileBrowserState<'static>;

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State,
    ) {
        if area.height < 8 {
            let message =
                Paragraph::new("Not enough space to display file list")
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
            message.render(area, buf);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Visual path display
                //Constraint::Length(3), // Editable path input
                Constraint::Min(1), // File list
            ])
            .split(area);

        //self.render_visual_path(buf, chunks[0]);
        //self.render_path_input(buf, chunks[1], state);
        self.render_integrated_path_input(buf, chunks[0], state);
        self.list_widget
            .render(chunks[1], buf, &mut state.list_state);

        if state.task_start_time.is_some() {
            self.render_loading(buf, area, state);
        }
    }
}

impl Drop for FileBrowserWidget {
    fn drop(&mut self) {
        // Close the channel to signal the background task to stop
        self.background_task.take();
    }
}
