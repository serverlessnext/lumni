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
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph,
    Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
    StatefulWidgetRef, Widget,
};
use tokio::sync::mpsc;

use super::{KeyTrack, ModalAction, TextArea, TextWindowTrait};
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

#[derive(Debug)]
pub enum BackgroundTaskResult {
    FileList(
        Result<Arc<Box<dyn Table + Send + Sync>>, ApplicationError>,
        Option<String>,
    ),
}

pub struct FileBrowserState<'a> {
    path_input: TextArea<'a>,
    selected_index: usize,
    displayed_index: usize,
    scroll_offset: usize,
    focus: FileBrowserFocus,
    filter_text: String,
    task_start_time: Option<Instant>,
}

impl<'a> Default for FileBrowserState<'a> {
    fn default() -> Self {
        let mut path_input = TextArea::new();
        path_input.text_set("", None).unwrap();
        Self {
            path_input,
            selected_index: 0,
            displayed_index: 0,
            scroll_offset: 0,
            focus: FileBrowserFocus::FileList,
            filter_text: String::new(),
            task_start_time: None,
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

        let mut path_input = TextArea::new();
        path_input.text_set("", None).unwrap();

        let widget = Self {
            base_path,
            current_path,
            file_table: None,
            background_task: Some(result_rx),
            operation_sender: op_tx,
        };

        let mut state = FileBrowserState::default();

        widget.start_list_files(&mut state);

        (widget, state)
    }

    pub fn get_selected_table_row(
        &self,
        state: &FileBrowserState,
    ) -> Option<TableRow> {
        if let Some(table) = &self.file_table {
            if let Some(row) = table.get_row(state.selected_index) {
                return Some(row);
            }
        }
        None
    }

    fn render_visual_path(&self, buf: &mut Buffer, area: Rect) {
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
                Style::default().fg(Color::Yellow),
                Style::default().fg(Color::Yellow),
            ),
            FileBrowserFocus::FileList => (Style::default(), Style::default()),
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

    fn update_selection(&self, state: &mut FileBrowserState) {
        if state.filter_text.is_empty() {
            state.displayed_index = usize::MAX; // No selection
            state.selected_index = usize::MAX; // No selection
        } else {
            state.displayed_index = 0;
            state.scroll_offset = 0;
        }
        self.update_selected_index(state);
    }

    fn has_selection(&self, state: &FileBrowserState) -> bool {
        state.displayed_index != usize::MAX
    }

    fn render_file_list(
        &self,
        buf: &mut Buffer,
        area: Rect,
        state: &mut FileBrowserState,
    ) {
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
        if !filtered_indices.is_empty() && state.displayed_index != usize::MAX {
            state.selected_index = filtered_indices
                [state.displayed_index.min(filtered_indices.len() - 1)];
        } else {
            state.selected_index = usize::MAX;
        }

        let list_height = area.height.saturating_sub(2) as usize;
        let total_items = items.len();

        // Adjust scroll_offset if necessary
        if state.displayed_index != usize::MAX {
            if state.displayed_index >= state.scroll_offset + list_height {
                state.scroll_offset =
                    state.displayed_index.saturating_sub(list_height) + 1;
            } else if state.displayed_index < state.scroll_offset {
                state.scroll_offset = state.displayed_index;
            }
        } else {
            // Reset scroll offset when there's no selection
            state.scroll_offset = 0;
        }

        // Ensure scroll_offset doesn't exceed max_scroll
        let max_scroll = total_items.saturating_sub(list_height);
        state.scroll_offset = state.scroll_offset.min(max_scroll);

        let items = items
            .into_iter()
            .skip(state.scroll_offset)
            .take(list_height)
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::default().title("Files").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        let mut list_state = ListState::default();
        if state.focus == FileBrowserFocus::FileList
            && state.displayed_index != usize::MAX
        {
            list_state.select(Some(
                state.displayed_index.saturating_sub(state.scroll_offset),
            ));
        } else {
            list_state.select(None);
        }

        StatefulWidget::render(list, area, buf, &mut list_state);

        if total_items > list_height {
            self.render_scrollbar(buf, area, total_items, list_height, state);
        }
    }

    fn render_scrollbar(
        &self,
        buf: &mut Buffer,
        area: Rect,
        total_items: usize,
        list_height: usize,
        state: &FileBrowserState,
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
        let scroll_position = (state.scroll_offset as f64 / max_scroll as f64
            * (list_height.saturating_sub(1)) as f64)
            .round() as usize;

        let mut scrollbar_state = ScrollbarState::new(list_height)
            .position(scroll_position.min(list_height.saturating_sub(1)));

        StatefulWidget::render(
            scrollbar,
            scrollbar_area,
            buf,
            &mut scrollbar_state,
        );
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
    ) -> Result<ModalAction, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Char(c) => {
                state.focus = FileBrowserFocus::PathInput;
                state.path_input.set_status_insert();
                state.path_input.process_edit_input(key_event)?;
                state.filter_text.push(c);
                self.update_selection(state);
            }
            KeyCode::Backspace => {
                if !state.filter_text.is_empty() {
                    state.filter_text.pop();
                    state.path_input.process_edit_input(key_event)?;
                    self.update_selection(state);
                } else {
                    self.go_up_directory(state);
                }
            }
            KeyCode::Down => {
                if state.focus == FileBrowserFocus::PathInput {
                    state.focus = FileBrowserFocus::FileList;
                    if state.displayed_index == usize::MAX {
                        state.displayed_index = 0;
                    }
                } else {
                    self.move_selection_down(state);
                }
            }
            KeyCode::Up => {
                if state.focus == FileBrowserFocus::FileList {
                    if state.displayed_index == 0 {
                        state.focus = FileBrowserFocus::PathInput;
                        state.displayed_index = usize::MAX;
                    } else {
                        self.move_selection_up(state);
                    }
                }
            }
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
            KeyCode::PageUp => {
                if state.focus == FileBrowserFocus::FileList {
                    self.page_up(state);
                }
            }
            KeyCode::PageDown => {
                if state.focus == FileBrowserFocus::FileList {
                    self.page_down(state);
                } else if state.focus == FileBrowserFocus::PathInput {
                    state.focus = FileBrowserFocus::FileList;
                    if state.displayed_index == usize::MAX {
                        state.displayed_index = 0;
                        self.update_selected_index(state);
                    }
                }
            }
            KeyCode::Esc => {
                state.focus = FileBrowserFocus::FileList;
                self.clear_path_input(state)?;
            }
            _ => {}
        }
        Ok(ModalAction::UpdateUI)
    }

    fn page_up(&self, state: &mut FileBrowserState) {
        let list_height = 10; // You might want to make this dynamic based on the actual view size
        if state.displayed_index == 0 {
            // If at the top, move focus to path input
            state.focus = FileBrowserFocus::PathInput;
            state.displayed_index = usize::MAX; // Indicate no selection
        } else if state.displayed_index > list_height {
            state.displayed_index -= list_height;
        } else {
            state.displayed_index = 0;
        }
        state.scroll_offset = state.scroll_offset.saturating_sub(list_height);
        self.update_selected_index(state);
    }

    fn page_down(&self, state: &mut FileBrowserState) {
        let list_height = 10; // You might want to make this dynamic based on the actual view size
        if let Some(table) = &self.file_table {
            let filtered_count = self.get_filtered_count(table, state);
            let max_index = filtered_count.saturating_sub(1);
            if state.displayed_index + list_height < max_index {
                state.displayed_index += list_height;
            } else {
                state.displayed_index = max_index;
            }
            let max_scroll = filtered_count.saturating_sub(list_height);
            state.scroll_offset =
                (state.scroll_offset + list_height).min(max_scroll);
            self.update_selected_index(state);
        }
    }

    fn get_filtered_count(
        &self,
        table: &Arc<Box<dyn Table + Send + Sync>>,
        state: &FileBrowserState,
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
                            .starts_with(&state.filter_text.to_lowercase())
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .count()
    }

    fn update_selected_index(&self, state: &mut FileBrowserState) {
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
                                .starts_with(&state.filter_text.to_lowercase())
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .collect();

            if !filtered_indices.is_empty() {
                state.selected_index = filtered_indices
                    [state.displayed_index.min(filtered_indices.len() - 1)];
            } else {
                state.selected_index = usize::MAX;
            }
        }
    }

    fn get_selected_path(&self, state: &FileBrowserState) -> Option<PathBuf> {
        if let Some(table) = &self.file_table {
            if let Some(row) = table.get_row(state.selected_index) {
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

    fn start_list_files(&self, state: &mut FileBrowserState) {
        let _ = self.operation_sender.try_send(FileOperation::ListFiles(
            self.current_path.to_string_lossy().into_owned(),
            None,
        ));
        state.task_start_time = Some(Instant::now());
        state.filter_text.clear();
        state.selected_index = 0;
        state.displayed_index = 0;
        state.scroll_offset = 0;
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
            state.displayed_index = 0;
            state.scroll_offset = 0;
        }
    }

    fn move_selection_up(&self, state: &mut FileBrowserState) {
        if state.displayed_index != usize::MAX && state.displayed_index > 0 {
            state.displayed_index -= 1;
        }
    }

    fn move_selection_down(&self, state: &mut FileBrowserState) {
        if let Some(table) = &self.file_table {
            let filtered_count = self.get_filtered_count(table, state);
            let max_index = filtered_count.saturating_sub(1);
            if state.displayed_index != usize::MAX
                && state.displayed_index < max_index
            {
                state.displayed_index += 1;
            } else if state.displayed_index == usize::MAX && filtered_count > 0
            {
                state.displayed_index = 0;
            }
        }
    }

    pub async fn poll_background_task(
        &mut self,
        state: &mut FileBrowserState<'static>,
    ) -> Result<(), ApplicationError> {
        if let Some(ref mut rx) = self.background_task {
            match rx.try_recv() {
                Ok(result) => {
                    self.handle_background_task_result(result, state).await?;
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
        state: &mut FileBrowserState<'static>,
    ) -> Result<(), ApplicationError> {
        match result {
            BackgroundTaskResult::FileList(result, file_to_select) => {
                state.task_start_time = None;
                match result {
                    Ok(table) => {
                        self.file_table = Some(table);
                        if let Some(name) = file_to_select {
                            self.select_file_by_name(&name, state);
                        } else {
                            state.selected_index = 0;
                            state.displayed_index = 0;
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn select_file_by_name(&self, name: &str, state: &mut FileBrowserState) {
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
                        state.selected_index = index;
                        state.displayed_index = index;
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
                Constraint::Length(3), // Editable path input
                Constraint::Min(1),    // File list
            ])
            .split(area);

        self.render_visual_path(buf, chunks[0]);
        self.render_path_input(buf, chunks[1], state);
        self.render_file_list(buf, chunks[2], state);

        if state.task_start_time.is_some() {
            self.render_loading(buf, area, state);
        }
    }
}
