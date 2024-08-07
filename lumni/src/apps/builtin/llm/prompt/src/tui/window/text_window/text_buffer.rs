use lumni::api::error::ApplicationError;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::{
    CodeBlock, CodeBlockLine, CodeBlockLineType, Cursor, LineType, MoveCursor,
    TextDisplay, TextDocumentTrait, TextLine,
};
use crate::external as lumni;

#[derive(Debug, Clone)]
pub struct TextBuffer<'a, T: TextDocumentTrait> {
    text_document: T,              // text buffer
    placeholder: String,           // placeholder text
    text_display: TextDisplay<'a>, // text (e.g. wrapped,  highlighted) for display
    cursor: Cursor,
    code_blocks: Vec<CodeBlock>, // code blocks
}

impl<'a, T: TextDocumentTrait> TextBuffer<'a, T> {
    pub fn new(text_document: T) -> Self {
        Self {
            text_document,
            placeholder: String::new(),
            text_display: TextDisplay::new(0),
            cursor: Cursor::new(),
            code_blocks: Vec::new(),
        }
    }

    pub fn set_placeholder(&mut self, text: &str) {
        if self.placeholder != text {
            self.placeholder = text.to_string();
            if self.text_document.is_empty() {
                // trigger display update if placeholder text changed,
                // and the text buffer is empty
                self.update_display_text();
            }
        }
    }

    pub fn set_cursor_visibility(&mut self, visible: bool) {
        if self.cursor.show_cursor() != visible {
            self.cursor.set_visibility(visible);
            // update display when cursor visibility changes
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
        // update display
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
        // Get the current cursor position in the underlying (unwrapped) text buffer
        let idx = self.cursor.real_position();
        self.text_document.insert(idx, text, style)?;
        self.update_display_text();

        // Calculate the number of newlines and the length of the last line segment
        let mut newlines = 0;
        let mut last_line_length = 0;
        for ch in text.chars() {
            if ch == '\n' {
                newlines += 1;
                last_line_length = 0; // Reset line length counter after each newline
            } else {
                last_line_length += 1; // Increment line length for non-newline characters
            }
        }

        if newlines > 0 {
            // Move the cursor to the end of the inserted text
            self.move_cursor(MoveCursor::Down(newlines), true);
            self.move_cursor(MoveCursor::StartOfLine, true); // Move to the start of the new line
            if last_line_length > 0 {
                // Then move right to the end of the inserted text on the last line
                self.move_cursor(MoveCursor::Right(last_line_length), true);
            }
        } else {
            // If no newlines, just move right
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
        // get current cursor position in the underlying (unwrapped) text buffer
        let idx = self.cursor.real_position();
        if char_count == 0 {
            return Ok(()); // nothing to delete
        }

        let start_idx = if include_cursor {
            idx //  start at the highlighed (cursor) character
        } else if idx > 0 {
            idx - 1 // start at the character before the cursor
        } else {
            return Ok(()); // nothing to delete
        };

        self.text_document.delete(start_idx, char_count)?;

        if include_cursor {
            // delete rightwards from the cursor
            // extra update is required to get correct text after deletion,
            self.update_display_text();

            // check if the cursor is at the end of the line
            if self.cursor.col as usize >= self.to_string().len() {
                self.move_cursor(MoveCursor::Left(char_count), false);
            }
        } else {
            // delete leftwards from the cursor
            self.move_cursor(MoveCursor::Left(char_count), true);
        }
        Ok(())
    }

    pub fn row_line_type(&self, row: usize) -> Option<LineType> {
        let lines = self.text_display.wrap_lines();
        if row >= lines.len() {
            return None; // out of bounds
        }
        lines[row].line_type
    }

    pub fn get_code_block(&self, ptr: u16) -> Option<&CodeBlock> {
        self.code_blocks.get(ptr as usize)
    }

    pub fn display_window_lines(&self, start: usize, end: usize) -> Vec<Line> {
        self.text_display.select_window_lines(start, end)
    }

    pub fn display_lines_len(&self) -> usize {
        self.text_display.wrap_lines().len()
    }

    pub fn yank_selected_text(&self) -> Option<String> {
        // check if selection is active
        if self.cursor.selection_enabled() {
            // get selection bounds
            let (start_row, start_col, end_row, end_col) =
                self.cursor.get_selection_bounds();
            let lines = self
                .text_document
                .get_text_lines_selection(start_row, Some(end_row));

            if let Some(lines) = lines {
                let mut selected_lines = Vec::new();

                // Iterate over the lines within the selection
                for (idx, line) in lines.iter().enumerate() {
                    let line_str = line.to_string();
                    if idx == 0 {
                        // First row: get the text from start_col to the end
                        selected_lines.push(line_str[start_col..].to_string());
                    } else if idx == lines.len() - 1 {
                        // Last row: get the text from 0 to end_col
                        let end_col_inclusive =
                            (end_col + 1).min(line_str.len());
                        selected_lines
                            .push(line_str[..end_col_inclusive].to_string());
                    } else {
                        // Middle row: take the whole line
                        selected_lines.push(line_str);
                    }
                }
                // Join the selected lines
                let selected_text = selected_lines.join("\n");
                return Some(selected_text);
            }
            return Some("".to_string());
        }
        None
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
            // cursor moved in the underlying text buffer
            // get the cursor position in the wrapped text display
            let (prev_display_col, prev_display_row) =
                self.text_display.get_column_row();

            // update the display text
            self.update_display_text();

            // get the cursor position in the wrapped text display after update,
            // this is used to determine if the cursor moved in the display
            let (post_display_col, post_display_row) =
                self.text_display.get_column_row();
            return (
                prev_display_col != post_display_col,
                prev_display_row != post_display_row,
            );
        }
        return (false, false);
    }

    pub fn set_selection_anchor(&mut self, enable: bool) {
        self.cursor.set_selection_anchor(enable);
        self.update_display_text();
    }

    fn mark_code_blocks(&mut self) {
        let mut in_code_block = false;
        let mut current_code_block_start: Option<u16> = None;
        let mut code_block_ptr = 0;

        self.code_blocks.clear();
        let reset = Style::reset();

        for (line_number, line) in
            self.text_display.wrap_lines.iter_mut().enumerate()
        {
            let line_number = line_number as u16;

            if in_code_block && line.background == reset.bg {
                // ensure code block does not persist across different text blocks
                in_code_block = false;
            }

            // check length first to avoid unnecessary comparison
            if line.length == 3 && line.line.to_string() == "```" {
                if in_code_block {
                    // end of code block
                    in_code_block = false;

                    if let Some(_) = current_code_block_start {
                        // close the last code block
                        if let Some(last_code_block) =
                            self.code_blocks.last_mut()
                        {
                            last_code_block.end = Some(line_number);
                        }
                    }

                    line.line_type = Some(LineType::Code(CodeBlockLine::new(
                        code_block_ptr,
                        CodeBlockLineType::End,
                    )));
                    code_block_ptr += 1; // increment for the next code block
                } else {
                    // start of code block
                    in_code_block = true;
                    current_code_block_start = Some(line_number);
                    self.code_blocks.push(CodeBlock {
                        start: line_number,
                        end: None,
                    });
                    line.line_type = Some(LineType::Code(CodeBlockLine::new(
                        code_block_ptr,
                        CodeBlockLineType::Start,
                    )));
                }
            } else {
                // mark line as code or text based on wether it is in a code block
                line.line_type = Some(if in_code_block {
                    // remove default background color if set
                    // keep other background colors (e.g. selection, cursor highlighting)
                    let bg_default = line.background;
                    for span in line.spans_mut() {
                        if span.style.bg == bg_default {
                            span.style.bg = None;
                        }
                    }
                    LineType::Code(CodeBlockLine::new(
                        code_block_ptr,
                        CodeBlockLineType::Line,
                    ))
                } else {
                    LineType::Text
                });
            }

            if !in_code_block {
                current_code_block_start = None;
            }
        }
        // TODO: for each codeblock, add syntax styling
    }

    pub fn update_display_text(&mut self) {
        self.text_document.update_if_modified();

        let mut text_lines = self.text_document.text_lines().to_vec();
        if text_lines.is_empty() && !self.placeholder.is_empty() {
            // placeholder text
            let style = Style::default().fg(Color::DarkGray);
            let mut line_styled = TextLine::new();
            line_styled.add_segment(self.placeholder.clone(), Some(style));
            text_lines.push(line_styled);
        }

        self.text_display.update(&text_lines, &self.cursor);

        self.mark_code_blocks();
        self.cursor.update_real_position(&text_lines);
        self.update_cursor_style();
    }

    fn update_cursor_style(&mut self) {
        if !self.cursor.show_cursor() {
            return;
        }
        // Retrieve the cursor's column and row in the wrapped display
        let (column, row) = self.text_display.update_column_row(&self.cursor);
        let mut line_column = column;

        if let Some(current_line) = self.text_display.wrap_lines.get_mut(row) {
            let line_length = current_line.length;
            let last_segment = current_line.last_segment;
            let spans = current_line.spans_mut();
            if line_column >= line_length && last_segment {
                spans
                    .push(Span::styled("_", Style::default().bg(Color::Green)));
            } else {
                // Iterate through spans to find the correct position
                let mut new_spans = Vec::new();
                let mut span_offset = 0;
                for span in spans.iter() {
                    let span_length = span.content.len();
                    if line_column < span_length {
                        let mut chars = span.content.chars();
                        let before = chars
                            .by_ref()
                            .take(line_column)
                            .collect::<String>();
                        let cursor_char = chars.next().unwrap_or(' '); // Safely get the cursor character or space if none
                        let after = chars.collect::<String>();

                        if !before.is_empty() {
                            new_spans.push(Span::styled(before, span.style));
                        }

                        new_spans.push(Span::styled(
                            cursor_char.to_string(),
                            span.style.bg(Color::Yellow),
                        ));

                        if !after.is_empty() {
                            new_spans.push(Span::styled(after, span.style));
                        }

                        // Add remaining spans as is
                        new_spans.extend(
                            spans.iter().skip(span_offset + 1).cloned(),
                        );
                        break;
                    } else {
                        new_spans.push(span.clone());
                        line_column -= span_length;
                    }

                    span_offset += 1;
                }
                *spans = new_spans;
            }
        }
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
        // decrement added count by 1 because get_text_lines_selection
        // slices the index range inclusively
        // e.g. to get n lines: end_row = start_row + n - 1
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
