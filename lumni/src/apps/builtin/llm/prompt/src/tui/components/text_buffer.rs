use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use textwrap::{wrap, Options, WordSplitter};

use super::cursor::{Cursor, MoveCursor};
use super::piece_table::PieceTable;

#[derive(Debug, Clone)]
pub struct TextDisplay<'a> {
    lines: Vec<Line<'a>>, // Text (e.g., wrapped, highlighted) for display
    trailing_spaces: Vec<usize>, // Number of trailing spaces to consider for cursor calculations
    display_width: usize,   // Width of the display area, used for wrapping
}

impl<'a> TextDisplay<'a> {
    pub fn new(display_width: usize) -> Self {
        TextDisplay {
            lines: Vec::new(),
            trailing_spaces: Vec::new(),
            display_width,
        }
    }

    pub fn lines(&self) -> &[Line<'a>] {
        &self.lines
    }

    pub fn lines_mut(&mut self) -> &mut Vec<Line<'a>> {
        &mut self.lines
    }

    pub fn get_trailing_spaces(&self, row: u16) -> usize {
        self.trailing_spaces.get(row as usize).cloned().unwrap_or(0)
    }

    pub fn width(&self) -> usize {
        self.display_width
    }

    pub fn push_line(&mut self, line: Line<'a>, trailing_spaces: usize) {
        self.lines.push(line);
        self.trailing_spaces.push(trailing_spaces);
    }

    pub fn set_display_width(&mut self, width: usize) {
        self.display_width = width;
    }

    // Get the maximum column of a specific row
    pub fn get_max_col(&self, row: u16) -> u16 {
        let line_len = self.lines
            .get(row as usize)
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.len() as u16)
                    .sum::<u16>()
            })
            .unwrap_or(0);

        // account for trailing spaces when calculating the line length    
        let spaces = *self.trailing_spaces.get(row as usize).unwrap_or(&0) as u16;
        line_len.saturating_add(spaces)
    }

    pub fn clear(&mut self) {
        self.lines.clear();
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

        // Move the cursor to the end of the inserted text
        if newlines > 0 {
            self.move_cursor(MoveCursor::Down(newlines as u16));
            self.move_cursor(MoveCursor::StartOfLine);  // Move to the start of the new line
            if last_line_length > 0 {
                // Then move right to the end of the inserted text on the last line
                self.move_cursor(MoveCursor::Right(last_line_length as u16));  
            }
        } else {
            self.move_cursor(MoveCursor::Right(text.len() as u16));  // If no newlines, just move right
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
        self.move_cursor(MoveCursor::Left(char_count as u16));
    }

    pub fn text_insert_commit(&mut self) -> String {
        self.text.commit_insert_cache()
    }

    pub fn display_text(&self) -> Vec<Line> {
        self.display.lines().to_vec()
    }

    pub fn display_text_len(&self) -> usize {
        self.display.lines().len()
    }

    pub fn selected_text(&self) -> &str {
        // Return the highlighted text - e.g. for copying to clipboard
        &self.selected_text
    }

    pub fn cursor_position(&self) -> (u16, u16) {
        (self.cursor.col, self.cursor.row)
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) -> (bool, bool) {
        let prev_col = self.cursor.col;
        let prev_row = self.cursor.row;

        self.cursor.move_cursor(direction, &self.display);

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
        let text = self.text.content();
        self.display.clear();
        self.selected_text.clear();
        let mut current_row = 0;

        // Number of characters added to the display text
        // this is required to calculate the real position in the text
        let mut added_characters = 0;

        let selection_bounds = self.get_selection_bounds();

        for line in text.split('\n') {
            let trailing_spaces = line.len() - line.trim_end_matches(' ').len();
            let wrapped_lines = self.wrap_text(line);
            if wrapped_lines.is_empty() {
                self.handle_empty_line(current_row);
                added_characters += 1; // account for the newline character
                current_row += 1; // move to next line
            } else {
                current_row = self.process_wrapped_lines(
                    wrapped_lines,
                    current_row,
                    &selection_bounds,
                    &mut added_characters,
                    trailing_spaces,
                );
            }
        }

        self.cursor
            .update_real_position(&self.display, added_characters);
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
        added_characters: &mut usize,
        trailing_spaces: usize, // trailing spaces on the original unwrapped line
    ) -> usize {
        let (start_row, start_col, end_row, end_col) = *selection_bounds;
        let mut local_row = current_row;

        for wrapped_line in wrapped_lines {
            let mut spans = Vec::new();
            let chars: Vec<char> = wrapped_line.chars().collect();

            // Track characters added for each line wrapped
            let original_line_length = chars.len();

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

            // Calculate added characters due to line wrapping
            let displayed_line_length =
                spans.iter().map(|span| span.content.len()).sum::<usize>();
            if displayed_line_length > original_line_length {
                *added_characters +=
                    displayed_line_length - original_line_length;
            }

            self.display.push_line(Line::from(spans), trailing_spaces);
            local_row += 1;
        }

        local_row
    }

    fn update_cursor_style(&mut self) {
        if self.cursor.show_cursor() {
            let row_index = self.cursor.row as usize;
            let line_length =
                self.display.get_max_col(self.cursor.row) as usize;
            let trailing_spaces = self.display.get_trailing_spaces(self.cursor.row);

            if let Some(current_line) =
                self.display.lines_mut().get_mut(row_index)
            {
                let cursor_position = self.cursor.col as usize;

                if cursor_position >= line_length {
                    // Cursor is at the end of the line
                    if trailing_spaces > 0 {
                        // Add trailing spaces back to the line (these were removed during wrapping)
                        let spaces = std::iter::repeat(' ')
                            .take(trailing_spaces)
                            .collect::<String>();
                        current_line.spans.push(Span::raw(spaces));
                    }

                    // Append one additional space for the cursor itself, and style it
                    current_line.spans.push(Span::styled(
                        " ",
                        Style::default().bg(Color::Yellow),
                    ));
                } else {
                    // Style the cursor's current position within the line
                    if cursor_position < current_line.spans.len() {
                        if let Some(span) =
                            current_line.spans.get_mut(cursor_position)
                        {
                            span.style = Style::default().bg(Color::Yellow);
                        }
                    }
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
        self.text.content()
    }
}
