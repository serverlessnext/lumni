use lumni::api::error::ApplicationError;
use ratatui::style::{Color, Style};
use ratatui::text::Line;

use super::{
    CodeBlock, Cursor, LineType, MoveCursor, TextDisplay, TextDocumentTrait,
    TextLine,
};
use crate::external as lumni;

#[derive(Debug, Clone)]
pub struct TextBuffer<'a, T: TextDocumentTrait> {
    text_document: T,
    placeholder: String,
    text_display: TextDisplay<'a>,
    cursor: Cursor,
    show_cursor: bool,
}

impl<'a, T: TextDocumentTrait> TextBuffer<'a, T> {
    pub fn new(text_document: T) -> Self {
        Self {
            text_document,
            placeholder: String::new(),
            text_display: TextDisplay::new(0),
            cursor: Cursor::new(),
            show_cursor: false,
        }
    }

    pub fn set_placeholder(&mut self, text: &str) {
        if self.placeholder != text {
            self.placeholder = text.to_string();
            if self.text_document.is_empty() {
                self.update_display_text();
            }
        }
    }

    pub fn set_cursor_visibility(&mut self, visible: bool) {
        if self.show_cursor != visible {
            self.show_cursor = visible;
            self.update_display_text();
        }
    }

    pub fn get_column_row(&self) -> (usize, usize) {
        self.text_display.get_column_row()
    }

    pub fn max_row_idx(&self) -> usize {
        self.text_document.max_row_idx()
    }

    pub fn is_empty(&self) -> bool {
        self.text_document.is_empty()
    }

    pub fn empty(&mut self) {
        self.text_display.clear();
        self.cursor.reset();
        self.text_document.empty();
        self.update_display_text();
    }

    pub fn set_width(&mut self, width: usize) {
        self.text_display.set_display_width(width);
    }

    pub fn text_insert_add(
        &mut self,
        text: &str,
        style: Option<Style>,
    ) -> Result<(), ApplicationError> {
        let idx = self.cursor.real_position();
        self.text_document.insert(idx, text, style)?;
        self.update_display_text();

        let mut newlines = 0;
        let mut last_line_length = 0;
        for ch in text.chars() {
            if ch == '\n' {
                newlines += 1;
                last_line_length = 0;
            } else {
                last_line_length += 1;
            }
        }

        if newlines > 0 {
            self.move_cursor(MoveCursor::Down(newlines), true);
            self.move_cursor(MoveCursor::StartOfLine, true);
            if last_line_length > 0 {
                self.move_cursor(MoveCursor::Right(last_line_length), true);
            }
        } else {
            self.move_cursor(MoveCursor::Right(text.len()), true);
        }
        Ok(())
    }

    pub fn text_append(&mut self, text: &str, style: Option<Style>) {
        self.text_document.append(text, style);
        self.update_display_text();
    }

    pub fn text_delete(
        &mut self,
        include_cursor: bool,
        char_count: usize,
    ) -> Result<(), ApplicationError> {
        let idx = self.cursor.real_position();
        if char_count == 0 {
            return Ok(());
        }

        let start_idx = if include_cursor {
            idx
        } else if idx > 0 {
            idx - 1
        } else {
            return Ok(());
        };

        self.text_document.delete(start_idx, char_count)?;

        if include_cursor {
            self.update_display_text();
            if self.cursor.col as usize >= self.to_string().len() {
                self.move_cursor(MoveCursor::Left(char_count), false);
            }
        } else {
            self.move_cursor(MoveCursor::Left(char_count), true);
        }
        Ok(())
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
                .text_document
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
                return Some(selected_text);
            }
            Some("".to_string())
        } else {
            None
        }
    }

    pub fn move_cursor(
        &mut self,
        direction: MoveCursor,
        edit_mode: bool,
    ) -> (bool, bool) {
        let prev_real_col = self.cursor.col;
        let prev_real_row = self.cursor.row;

        self.cursor
            .move_cursor(direction, &self.text_document, edit_mode);

        let real_column_changed = prev_real_col != self.cursor.col;
        let real_row_changed = prev_real_row != self.cursor.row;
        if real_column_changed || real_row_changed {
            let (prev_display_col, prev_display_row) =
                self.text_display.get_column_row();

            self.update_display_text();

            let (post_display_col, post_display_row) =
                self.text_display.get_column_row();
            return (
                prev_display_col != post_display_col,
                prev_display_row != post_display_row,
            );
        }
        (false, false)
    }

    pub fn set_selection_anchor(&mut self, enable: bool) {
        self.cursor.set_selection_anchor(enable);
        self.update_display_text();
    }

    pub fn update_display_text(&mut self) {
        self.text_document.update_if_modified();

        let mut text_lines = self.text_document.text_lines().to_vec();
        if text_lines.is_empty() && !self.placeholder.is_empty() {
            let style = Style::default().fg(Color::DarkGray);
            let mut line_styled = TextLine::new();
            line_styled.add_segment(self.placeholder.clone(), Some(style));
            text_lines.push(line_styled);
        }

        // Update cursor's real position before updating the display
        self.cursor.update_real_position(&text_lines);

        // Update the display, including cursor rendering
        self.text_display
            .update(&text_lines, &self.cursor, self.show_cursor);
    }

    pub fn undo(&mut self) -> Result<(), ApplicationError> {
        self.text_document.undo()?;
        self.update_display_text();
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), ApplicationError> {
        self.text_document.redo()?;
        self.update_display_text();
        Ok(())
    }

    pub fn to_string(&self) -> String {
        self.text_document.to_string()
    }

    pub fn yank_lines(&self, count: usize) -> Vec<String> {
        let start_row = self.cursor.row as usize;
        let end_row = start_row.saturating_add(count.saturating_sub(1));

        if let Some(text_lines) = self
            .text_document
            .get_text_lines_selection(start_row, Some(end_row))
        {
            text_lines.iter().map(|line| line.to_string()).collect()
        } else {
            Vec::new()
        }
    }
}
