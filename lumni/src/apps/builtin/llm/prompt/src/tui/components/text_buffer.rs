use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use textwrap::{wrap, Options, WordSplitter};

use super::cursor::{Cursor, MoveCursor};
use super::piece_table::PieceTable;

#[derive(Debug, Clone)]
pub struct TextDisplay<'a> {
    wrap_lines: Vec<Line<'a>>, // Text (e.g., wrapped, highlighted) for display
    trailing_spaces: Vec<usize>, // Number of trailing spaces to consider for cursor calculations
    display_width: usize,   // Width of the display area, used for wrapping
}

impl<'a> TextDisplay<'a> {
    pub fn new(display_width: usize) -> Self {
        TextDisplay {
            wrap_lines: Vec::new(),
            trailing_spaces: Vec::new(),
            display_width,
        }
    }

    pub fn wrap_lines(&self) -> &[Line<'a>] {
        &self.wrap_lines
    }


    pub fn wrap_lines_mut(&mut self) -> &mut Vec<Line<'a>> {
        &mut self.wrap_lines
    }

    pub fn get_trailing_spaces(&self, row: u16) -> usize {
        self.trailing_spaces.get(row as usize).cloned().unwrap_or(0)
    }

    pub fn width(&self) -> usize {
        self.display_width
    }

    pub fn push_line(&mut self, line: Line<'a>, trailing_spaces: usize) {
        self.wrap_lines.push(line);
        self.trailing_spaces.push(trailing_spaces);
    }

    pub fn set_display_width(&mut self, width: usize) {
        self.display_width = width;
    }

    pub fn clear(&mut self) {
        self.wrap_lines.clear();
        self.trailing_spaces.clear();
    }
}

#[derive(Debug, Clone)]
pub struct TextBuffer<'a> {
    text: PieceTable,         // text buffer
    display: TextDisplay<'a>, // text (e.g. wrapped,  highlighted) for display
    selected_text: String,    // currently selected text
    cursor: Cursor,
    is_editable: bool,
}

impl TextBuffer<'_> {
    pub fn new(is_editable: bool) -> Self {
        Self {
            text: PieceTable::new(""),
            display: TextDisplay::new(0),
            selected_text: String::new(),
            cursor: Cursor::new(0, 0, is_editable),
            is_editable,
        }
    }

    pub fn set_cursor_visibility(&mut self, visible: bool) {
        if self.cursor.show_cursor() != visible {
            self.cursor.set_visibility(visible);
            // update display when cursor visibility changes
            self.update_display_text();
        }
    }

    pub fn empty(&mut self) {
        self.display.clear();
        self.selected_text.clear();
        self.cursor = Cursor::new(0, 0, self.is_editable);
        self.text.empty();
        // update display
        self.update_display_text();
    }

    pub fn set_width(&mut self, width: usize) {
        self.display.set_display_width(width);
    }

    pub fn text_insert_add(&mut self, text: &str) {
        // Get the current cursor position in the underlying (unwrapped) text buffer
        let idx = self.cursor.real_position();

        self.text.cache_insert(text, Some(idx));
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
            self.move_cursor(MoveCursor::Down(newlines as u16), true);
            self.move_cursor(MoveCursor::StartOfLine, true);  // Move to the start of the new line
            if last_line_length > 0 {
                // Then move right to the end of the inserted text on the last line
                self.move_cursor(MoveCursor::Right(last_line_length as u16), true);  
            }
        } else {
            // If no newlines, just move right
            self.move_cursor(MoveCursor::Right(text.len() as u16), true);  
        }
    }

    pub fn text_delete(&mut self, include_cursor: bool, char_count: usize) {
        // get current cursor position in the underlying (unwrapped) text buffer
        let idx = self.cursor.real_position();
        if char_count == 0 {
            return; // nothing to delete
        }

        let start_idx = if include_cursor {
            idx //  start at the highlighed (cursor) character
        } else if idx > 0 {
            idx - 1 // start at the character before the cursor
        } else {
            return;
        };

        self.text.delete(start_idx, char_count);

        if include_cursor {
            // delete rightwards from the cursor
            // extra update is required to get correct text after deletion,
            self.update_display_text();

            // check if the cursor is at the end of the line
            if self.cursor.col as usize >= self.to_string().len() {
                self.move_cursor(MoveCursor::Left(char_count as u16), false);
            }
        } else {
            // delete leftwards from the cursor
            self.move_cursor(MoveCursor::Left(char_count as u16), true);
        }
    }

    pub fn text_insert_commit(&mut self) -> String {
        self.text.commit_insert_cache()
    }

    pub fn display_text(&self) -> Vec<Line> {
        self.display.wrap_lines().to_vec()
    }

    pub fn display_text_len(&self) -> usize {
        self.display.wrap_lines().len()
    }

    pub fn selected_text(&self) -> &str {
        // Return the highlighted text - e.g. for copying to clipboard
        &self.selected_text
    }

    pub fn move_cursor(&mut self, direction: MoveCursor, edit_mode: bool) -> (bool, bool) {
        let prev_col = self.cursor.col;
        let prev_row = self.cursor.row;

        self.cursor.move_cursor(direction, &self.text.lines(), edit_mode);

        let column_changed = prev_col != self.cursor.col;
        let row_changed = prev_row != self.cursor.row;
        if column_changed || row_changed {
            // update the display text to reflect the change
            self.update_display_text();
        }
        (column_changed, row_changed)
    }

    pub fn set_selection(&mut self, enable: bool) {
        self.cursor.set_selection(enable);
        self.update_display_text();
    }

    pub fn update_display_text(&mut self) {
        self.text.update_lines();
        self.display.clear();
        self.selected_text.clear();
        let mut current_row = 0;

        let selection_bounds = self.get_selection_bounds();
        let text_lines = self.text.lines().to_vec();

        for line in &text_lines {
            let trailing_spaces = line.len() - line.trim_end_matches(' ').len();
            let wrapped_lines = self.wrap_text(&line);

            if wrapped_lines.is_empty() {
                self.handle_empty_line(current_row);
                current_row += 1; // move to next line
            } else {
                current_row = self.process_wrapped_lines(
                    wrapped_lines,
                    current_row,
                    &selection_bounds,
                    trailing_spaces,
                );
            }
        }

        // recompute added_characters by comparing displayed text length with the underlying text
        self.cursor
            .update_real_position(&text_lines);
        self.update_cursor_style();
    }

    fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        if self.cursor.selection_enabled() {
            self.cursor.get_selection_bounds()
        } else {
            (usize::MAX, usize::MAX, usize::MIN, usize::MIN) // No highlighting
        }
    }

    fn wrap_text(&self, line: &str) -> Vec<String> {
        // check for space on end of line
        wrap(
            line,
            Options::new(self.display.width())
                .word_splitter(WordSplitter::NoHyphenation),
        )
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect()
    }

    fn handle_empty_line(&mut self, current_row: usize) {
        if current_row == self.cursor.row as usize && self.cursor.show_cursor()
        {
            let span = Span::styled(" ", Style::default().bg(Color::Blue));
            self.display.push_line(Line::from(span), 0);
        } else {
            self.display.push_line(Line::from(Span::raw("")), 0);
        }
    }

    fn process_wrapped_lines(
        &mut self,
        wrapped_lines: Vec<String>,
        current_row: usize,
        selection_bounds: &(usize, usize, usize, usize),
        trailing_spaces: usize, // trailing spaces on the original unwrapped line
    ) -> usize {
        let (start_row, start_col, end_row, end_col) = *selection_bounds;
        let mut local_row = current_row;
        for wrapped_line in wrapped_lines {
            let mut spans = Vec::new();
            let chars: Vec<char> = wrapped_line.chars().collect();

            for (j, ch) in chars.into_iter().enumerate() {
                let should_select = self.cursor.show_cursor()
                    && self.cursor.should_select(
                        local_row, j, start_row, start_col, end_row, end_col,
                    );

                if should_select {
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().bg(Color::Blue),
                    ));
                    self.selected_text.push(ch);
                } else {
                    spans.push(Span::raw(ch.to_string()));
                }
            }
            self.display.push_line(Line::from(spans), trailing_spaces);
            local_row += 1;
        }
        local_row
    }

    fn update_cursor_style(&mut self) {
        if !self.cursor.show_cursor() {
            return;
        }

        // Retrieve the cursor's column and row in the wrapped display
        let (column, row) = self.display_column_row();
        let trailing_spaces = self.display.get_trailing_spaces(row as u16);

        if let Some(current_line) = self.display.wrap_lines_mut().get_mut(row) {
            if column >= current_line.spans.len() {
                // The cursor is at the end of the line or beyond the last character

                // Add trailing spaces back to the line if necessary
                if trailing_spaces > 0 && column > current_line.spans.len() {
                    let spaces = std::iter::repeat(' ')
                        .take(trailing_spaces)
                        .collect::<String>();
                    current_line.spans.push(Span::raw(spaces));
                }

                // Append a styled space for the cursor itself
                current_line.spans.push(Span::styled(" ", Style::default().bg(Color::Yellow)));
            } else {
                // Style the cursor's current position within the line
                if let Some(span) = current_line.spans.get_mut(column) {
                    span.style = Style::default().bg(Color::Yellow);
                }
            }
        }
    }


    pub fn undo(&mut self) {
        self.text.undo();
        self.update_display_text();
    }

    pub fn redo(&mut self) {
        self.text.redo();
        self.update_display_text();
    }

    pub fn to_string(&self) -> String {
        self.text.lines().join("\n")
    }

    pub fn display_column_row(&self) -> (usize, usize) {
        // Get the current row in the wrapped text display based on the cursor position
        let cursor_position = self.cursor.real_position();

        let mut wrap_position = 0;
        for (row, line) in self.display.wrap_lines().iter().enumerate() {
            let mut line_length = line
                .spans
                .iter()
                .map(|span| span.content.len())
                .sum::<usize>();
            let trailing_spaces = self.display.get_trailing_spaces(row as u16);
            line_length += trailing_spaces;

            if wrap_position + line_length >= cursor_position {
                // Cursor is on this line
                let column = cursor_position - wrap_position;
                return (column, row);
            }
            wrap_position += line_length + 1; // account for newline character
        }
        (0, 0)  // default to (0, 0) if cursor is not found
    }
}
