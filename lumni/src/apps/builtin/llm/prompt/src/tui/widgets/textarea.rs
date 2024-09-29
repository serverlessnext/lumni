use crossterm::event::{KeyCode, KeyEvent};
use lumni::api::error::ApplicationError;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
    StatefulWidgetRef, Widget,
};
use ratatui::Frame;

use super::{
    CodeBlock, Cursor, KeyTrack, LineType, MoveCursor, ReadDocument,
    ReadWriteDocument, TextDisplay, TextDocumentTrait, TextLine,
};
pub use crate::external as lumni;

#[derive(Debug, Clone)]
pub struct TextArea<T: TextDocumentTrait> {
    widget: TextAreaWidget<T>,
    state: TextAreaState<'static, T>,
}

impl<T: TextDocumentTrait> TextArea<T> {
    pub fn new() -> Self
    where
        T: Default,
    {
        Self {
            widget: TextAreaWidget::new(),
            state: TextAreaState::new(T::default()),
        }
    }

    pub fn with_state(state: TextAreaState<'static, T>) -> Self {
        Self {
            widget: TextAreaWidget::new(),
            state,
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        self.state.handle_key_event(key_event);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        f.render_stateful_widget(&self.widget, area, &mut self.state);
    }
}

impl TextArea<ReadDocument> {
    pub fn with_read_document(text: Option<Vec<TextLine>>) -> Self {
        Self::with_state(TextAreaState::with_read_document(text))
    }
}

impl TextArea<ReadWriteDocument> {
    pub fn with_read_write_document(text: Option<Vec<TextLine>>) -> Self {
        Self::with_state(TextAreaState::with_read_write_document(text))
    }
}
#[derive(Debug, Clone)]
pub struct TextAreaWidget<T: TextDocumentTrait>(std::marker::PhantomData<T>);

impl<T: TextDocumentTrait> TextAreaWidget<T> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }

    fn render_scrollbar(
        &self,
        buf: &mut Buffer,
        area: Rect,
        state: &TextAreaState<'_, T>,
    ) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        let total_lines = state.display_lines_len();
        let viewport_height = state.viewport_height;

        let scrollable_area =
            total_lines.saturating_sub(viewport_height).max(1);

        let position = ((state.scroll_offset as f64 / scrollable_area as f64)
            * scrollable_area as f64)
            .round() as usize;

        let mut scrollbar_state =
            ScrollbarState::new(scrollable_area).position(position);

        StatefulWidget::render(
            scrollbar,
            scrollbar_area,
            buf,
            &mut scrollbar_state,
        );
    }
}

impl<T: TextDocumentTrait> StatefulWidget for &TextAreaWidget<T> {
    type State = TextAreaState<'static, T>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidgetRef::render_ref(&self, area, buf, state)
    }
}

impl<T: TextDocumentTrait> StatefulWidgetRef for &TextAreaWidget<T> {
    type State = TextAreaState<'static, T>;

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State,
    ) {
        state.set_width(area.width as usize);
        state.set_viewport_height(area.height as usize);
        state.update_display_text();

        let total_lines = state.display_lines_len();
        let viewport_height = area.height as usize;

        if total_lines > viewport_height {
            let max_scroll = total_lines - viewport_height;
            state.scroll_offset = state.scroll_offset.min(max_scroll);
        } else {
            state.scroll_offset = 0;
        }

        let visible_text = state.display_window_lines(
            state.scroll_offset,
            state.scroll_offset + viewport_height,
        );

        let paragraph = Paragraph::new(visible_text)
            .style(ratatui::style::Style::default());
        Widget::render(paragraph, area, buf);

        if total_lines > viewport_height {
            self.render_scrollbar(buf, area, state);
        }
    }
}

use std::cmp;

#[derive(Debug, Clone, PartialEq)]
pub enum TextAreaMode {
    Normal,
    Insert,
    Visual,
}

#[derive(Debug, Clone)]
pub struct TextAreaState<'a, T: TextDocumentTrait> {
    document: T,
    cursor: Cursor,
    text_display: TextDisplay<'a>,
    key_track: KeyTrack,
    scroll_offset: usize,
    viewport_height: usize,
    mode: TextAreaMode,
    show_cursor: bool,
    placeholder: String,
}

impl<'a, T: TextDocumentTrait> TextAreaState<'a, T> {
    pub fn new(document: T) -> Self {
        Self {
            document,
            cursor: Cursor::new(),
            text_display: TextDisplay::new(0),
            key_track: KeyTrack::new(),
            scroll_offset: 0,
            viewport_height: 0,
            mode: TextAreaMode::Normal,
            show_cursor: true,
            placeholder: String::new(),
        }
    }

    pub fn document(&self) -> &T {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut T {
        &mut self.document
    }

    pub fn mode(&self) -> &TextAreaMode {
        &self.mode
    }

    pub fn set_mode(&mut self, mode: TextAreaMode) {
        self.mode = mode;
        match self.mode {
            TextAreaMode::Insert => {
                self.show_cursor = true;
                self.cursor.set_selection_anchor(false);
            }
            TextAreaMode::Normal => {
                self.show_cursor = true;
                self.cursor.set_selection_anchor(false);
            }
            TextAreaMode::Visual => {
                self.show_cursor = true;
                self.cursor.set_selection_anchor(true);
            }
        }
        self.update_display_text();
    }

    pub fn toggle_cursor_visibility(&mut self) {
        self.show_cursor = !self.show_cursor;
        self.update_display_text();
    }

    pub fn set_placeholder(&mut self, text: &str) {
        if self.placeholder != text {
            self.placeholder = text.to_string();
            if self.document.is_empty() {
                self.update_display_text();
            }
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        self.key_track.process_key(key_event);

        match self.mode {
            TextAreaMode::Normal => self.handle_normal_mode(key_event),
            TextAreaMode::Insert => self.handle_insert_mode(key_event),
            TextAreaMode::Visual => self.handle_visual_mode(key_event),
        }

        self.update_scroll();
    }

    fn handle_normal_mode(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('i') => self.set_mode(TextAreaMode::Insert),
            KeyCode::Char('v') => self.set_mode(TextAreaMode::Visual),
            KeyCode::Up => {
                self.move_cursor(MoveCursor::Up(1));
            }
            KeyCode::Down => {
                self.move_cursor(MoveCursor::Down(1));
            }
            KeyCode::Left => {
                self.move_cursor(MoveCursor::Left(1));
            }
            KeyCode::Right => {
                self.move_cursor(MoveCursor::Right(1));
            }
            KeyCode::Home => {
                self.move_cursor(MoveCursor::StartOfLine);
            }
            KeyCode::End => {
                self.move_cursor(MoveCursor::EndOfLine);
            }
            KeyCode::PageUp => self.scroll_up(self.viewport_height),
            KeyCode::PageDown => self.scroll_down(self.viewport_height),
            _ => {}
        }
    }

    fn handle_insert_mode(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.set_mode(TextAreaMode::Normal),
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Delete => self.handle_delete(),
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Char(c) => self.handle_char(c),
            KeyCode::Tab => self.handle_tab(),
            KeyCode::Left => {
                self.move_cursor(MoveCursor::Left(1));
            }
            KeyCode::Right => {
                self.move_cursor(MoveCursor::Right(1));
            }
            KeyCode::Up => {
                self.move_cursor(MoveCursor::Up(1));
            }
            KeyCode::Down => {
                self.move_cursor(MoveCursor::Down(1));
            }
            KeyCode::Home => {
                self.move_cursor(MoveCursor::StartOfLine);
            }
            KeyCode::End => {
                self.move_cursor(MoveCursor::EndOfLine);
            }
            _ => {}
        }
    }

    fn handle_visual_mode(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.set_mode(TextAreaMode::Normal),
            KeyCode::Up => {
                self.move_cursor(MoveCursor::Up(1));
            }
            KeyCode::Down => {
                self.move_cursor(MoveCursor::Down(1));
            }
            KeyCode::Left => {
                self.move_cursor(MoveCursor::Left(1));
            }
            KeyCode::Right => {
                self.move_cursor(MoveCursor::Right(1));
            }
            KeyCode::Home => {
                self.move_cursor(MoveCursor::StartOfLine);
            }
            KeyCode::End => {
                self.move_cursor(MoveCursor::EndOfLine);
            }
            KeyCode::PageUp => {
                self.move_cursor(MoveCursor::Up(self.viewport_height));
            }
            KeyCode::PageDown => {
                self.move_cursor(MoveCursor::Down(self.viewport_height));
            }
            _ => {}
        }
    }

    fn move_cursor(&mut self, direction: MoveCursor) -> (bool, bool) {
        let prev_real_col = self.cursor.col;
        let prev_real_row = self.cursor.row;

        self.cursor.move_cursor(direction, &self.document, true);

        let real_column_changed = prev_real_col != self.cursor.col;
        let real_row_changed = prev_real_row != self.cursor.row;

        if real_column_changed || real_row_changed {
            let (prev_display_col, prev_display_row) =
                self.text_display.get_column_row();
            self.update_display_text();
            let (post_display_col, post_display_row) =
                self.text_display.get_column_row();
            (
                prev_display_col != post_display_col,
                prev_display_row != post_display_row,
            )
        } else {
            (false, false)
        }
    }

    fn handle_backspace(&mut self) {
        let idx = self.cursor.real_position();
        if idx > 0 {
            if let Err(e) = self.document.delete(idx - 1, 1) {
                eprintln!("Error handling backspace: {:?}", e);
            } else {
                self.move_cursor(MoveCursor::Left(1));
            }
        }
    }

    fn handle_delete(&mut self) {
        let idx = self.cursor.real_position();
        if let Err(e) = self.document.delete(idx, 1) {
            eprintln!("Error handling delete: {:?}", e);
        } else {
            self.update_display_text();
        }
    }

    fn handle_enter(&mut self) {
        let idx = self.cursor.real_position();
        if let Err(e) = self.document.insert(idx, "\n", None) {
            eprintln!("Error handling enter: {:?}", e);
        } else {
            self.move_cursor(MoveCursor::Right(1));
        }
    }

    fn handle_char(&mut self, c: char) {
        let idx = self.cursor.real_position();
        if let Err(e) = self.document.insert(idx, &c.to_string(), None) {
            eprintln!("Error handling character input: {:?}", e);
        } else {
            self.move_cursor(MoveCursor::Right(1));
        }
        self.update_display_text();
    }

    fn handle_tab(&mut self) {
        let idx = self.cursor.real_position();
        if let Err(e) = self.document.insert(idx, "    ", None) {
            eprintln!("Error handling tab: {:?}", e);
        } else {
            self.move_cursor(MoveCursor::Right(4));
        }
    }

    fn update_scroll(&mut self) {
        let (_, cursor_row) = self.text_display.get_column_row();
        if cursor_row < self.scroll_offset {
            self.scroll_offset = cursor_row;
        } else if cursor_row >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = cursor_row - self.viewport_height + 1;
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: usize) {
        let total_lines = self.text_display.wrap_lines().len();
        let max_scroll = total_lines.saturating_sub(self.viewport_height);
        self.scroll_offset = cmp::min(self.scroll_offset + amount, max_scroll);
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }

    pub fn set_width(&mut self, width: usize) {
        self.text_display.set_display_width(width);
    }

    pub fn update_display_text(&mut self) {
        self.document.update_if_modified();

        let mut text_lines = self.document.text_lines().to_vec();
        if text_lines.is_empty() && !self.placeholder.is_empty() {
            let style = Style::default().fg(Color::DarkGray);
            let mut line_styled = TextLine::new();
            line_styled.add_segment(self.placeholder.clone(), Some(style));
            text_lines.push(line_styled);
        }

        self.cursor.update_real_position(&text_lines);
        self.text_display
            .update(&text_lines, &self.cursor, self.show_cursor);
    }

    pub fn display_window_lines(&self, start: usize, end: usize) -> Vec<Line> {
        self.text_display.select_window_lines(start, end)
    }

    pub fn display_lines_len(&self) -> usize {
        self.text_display.wrap_lines().len()
    }

    pub fn yank_selected_text(&self) -> Option<String> {
        if self.cursor.selection_enabled() {
            let (start_row, start_col, end_row, end_col) =
                self.cursor.get_selection_bounds();
            let lines = self
                .document
                .get_text_lines_selection(start_row, Some(end_row));

            if let Some(lines) = lines {
                let mut selected_lines = Vec::new();

                for (idx, line) in lines.iter().enumerate() {
                    let line_str = line.to_string();
                    if idx == 0 {
                        selected_lines.push(line_str[start_col..].to_string());
                    } else if idx == lines.len() - 1 {
                        let end_col_inclusive =
                            (end_col + 1).min(line_str.len());
                        selected_lines
                            .push(line_str[..end_col_inclusive].to_string());
                    } else {
                        selected_lines.push(line_str);
                    }
                }
                let selected_text = selected_lines.join("\n");
                Some(selected_text)
            } else {
                Some("".to_string())
            }
        } else {
            None
        }
    }

    pub fn set_selection_anchor(&mut self, enable: bool) {
        self.cursor.set_selection_anchor(enable);
        self.update_display_text();
    }

    pub fn undo(&mut self) -> Result<(), ApplicationError> {
        self.document.undo()?;
        self.update_display_text();
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), ApplicationError> {
        self.document.redo()?;
        self.update_display_text();
        Ok(())
    }

    pub fn to_string(&self) -> String {
        self.document.to_string()
    }

    pub fn yank_lines(&self, count: usize) -> Vec<String> {
        let start_row = self.cursor.row as usize;
        let end_row = start_row.saturating_add(count.saturating_sub(1));

        if let Some(text_lines) = self
            .document
            .get_text_lines_selection(start_row, Some(end_row))
        {
            text_lines.iter().map(|line| line.to_string()).collect()
        } else {
            Vec::new()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.document.is_empty()
    }

    pub fn empty(&mut self) {
        self.text_display.clear();
        self.cursor.reset();
        self.document.empty();
        self.update_display_text();
    }

    pub fn get_column_row(&self) -> (usize, usize) {
        self.text_display.get_column_row()
    }

    pub fn max_row_idx(&self) -> usize {
        self.document.max_row_idx()
    }

    pub fn row_line_type(&self, row: usize) -> Option<LineType> {
        self.text_display
            .wrap_lines()
            .get(row)
            .and_then(|line| line.line_type)
    }

    pub fn get_code_block(&self, ptr: u16) -> Option<&CodeBlock> {
        self.text_display.get_code_block(ptr)
    }
}

impl<'a> TextAreaState<'a, ReadDocument> {
    pub fn with_read_document(text: Option<Vec<TextLine>>) -> Self {
        let document = if let Some(text) = text {
            ReadDocument::from_text(text)
        } else {
            ReadDocument::new()
        };
        Self::new(document)
    }
}

impl<'a> TextAreaState<'a, ReadWriteDocument> {
    pub fn with_read_write_document(text: Option<Vec<TextLine>>) -> Self {
        let document = if let Some(text) = text {
            ReadWriteDocument::from_text(text)
        } else {
            ReadWriteDocument::new()
        };
        Self::new(document)
    }
}
