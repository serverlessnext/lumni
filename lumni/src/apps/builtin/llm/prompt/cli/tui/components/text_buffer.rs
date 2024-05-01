use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use textwrap::{wrap, Options, WordSplitter};

use super::cursor::{Cursor, MoveCursor};
use super::piece_table::PieceTable;
use super::window_type::WindowStyle;

#[derive(Debug, Clone)]
pub struct TextDisplay<'a> {
    lines: Vec<Line<'a>>,  // Text (e.g., wrapped, highlighted) for display
    trailing_spaces: usize, // Number of trailing spaces to consider for cursor calculations
    display_width: usize,   // Width of the display area, used for wrapping
}

impl<'a> TextDisplay<'a> {
    pub fn new(display_width: usize) -> Self {
        TextDisplay {
            lines: Vec::new(),
            trailing_spaces: 0,
            display_width,
        }
    }

    pub fn lines(&self) -> &[Line<'a>] {
        &self.lines
    }

    pub fn lines_mut(&mut self) -> &mut Vec<Line<'a>> {
        &mut self.lines
    }

    pub fn width(&self) -> usize {
        self.display_width
    }

    pub fn push_line(&mut self, line: Line<'a>) {
        self.lines.push(line);
    }

    pub fn set_trailing_spaces(&mut self, count: usize) {
        self.trailing_spaces = count;
    }

    pub fn set_display_width(&mut self, width: usize) {
        self.display_width = width;
    }

    // Get the maximum column of a specific row
    pub fn get_max_col(&self, row: u16) -> u16 {
        self.lines.get(row as usize)
            .map(|line| line.spans.iter().map(|span| span.content.len() as u16).sum::<u16>())
            .unwrap_or(0)
            .saturating_add(self.trailing_spaces as u16)  // Include trailing spaces
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

#[derive(Debug, Clone)]
pub struct TextBuffer<'a> {
    text: PieceTable,            // text buffer
    display: TextDisplay<'a>,    // text (e.g. wrapped,  highlighted) for display
    selected_text: String,       // currently selected text
    cursor: Cursor,
}

impl TextBuffer<'_> {
    pub fn new() -> Self {
        Self {
            text: PieceTable::new(""),
            display: TextDisplay::new(0),
            selected_text: String::new(),
            cursor: Cursor::new(0, 0),
        }
    }

    pub fn empty(&mut self) {
        self.display.clear();
        self.selected_text.clear();
        self.cursor.reset();
        self.text.empty();
        // update display
        self.update_display_text();
    }

    pub fn set_width(&mut self, width: usize) {
        self.display.set_display_width(width);
    }

    pub fn set_cursor_style(&mut self, style: WindowStyle) {
        self.cursor.set_style(style);
    }

    pub fn text_insert_add(&mut self, text: &str) {
        // get current cursor position in the underlying (unwrapped) text buffer
        let idx = self.cursor.real_position();
        self.text.cache_insert(text, Some(idx));
        self.update_display_text();
        self.move_cursor(MoveCursor::Right(text.len() as u16));
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
    
        let end_idx =
            std::cmp::min(start_idx + char_count, self.text.content().len());

        if char_count == 1 {
            // handle backspace separately as it has different user expectations,
            // particulary on deleting of characters at the end of the line

            // get additional character before the start_idx to check for starting newline
            let deleted_text = &self.text.content()[start_idx.saturating_sub(1)..end_idx];

            if deleted_text.ends_with('\n') {
                if deleted_text.starts_with("\n") {
                    // case with continguos newlines -- delete only given char_count
                    self.text.delete(start_idx, char_count);
                } else {
                    // delete last character(s) from previous line + newline
                    // move index one character back to include the additional newline delete
                    self.text.delete(start_idx.saturating_sub(1), char_count + 1);
                }
            } else {
                // delete character within the line
                self.text.delete(start_idx, char_count);
            }
        } else {
            // delete the selected text
            // TODO: should still test multi-line delete
            self.text.delete(start_idx, char_count);
        }
        
        // Move cursor appropriately
        self.cursor.move_cursor(
            MoveCursor::Left(char_count as u16),
            &self.display,
        );

        self.update_display_text();
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

        self.cursor.move_cursor(
            direction,
            &self.display,
        );

        let column_changed = prev_col != self.cursor.col;
        let row_changed = prev_row != self.cursor.row;
        if self.cursor.show_cursor() && (column_changed || row_changed) {
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
                );
            }
        }
        let trailing_spaces = text.len() - text.trim_end_matches(' ').len();
        self.display.set_trailing_spaces(trailing_spaces);

        self.cursor
            .update_real_position(&self.display, added_characters);
        self.update_cursor_style_in_insert_mode();
    }

    fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        if self.cursor.selection_enabled() {
            self.cursor.get_selection_bounds()
        } else {
            (usize::MAX, usize::MAX, usize::MIN, usize::MIN) // No highlighting
        }
    }

    fn wrap_text(&self, line: &str) -> Vec<String> {
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
        if current_row == self.cursor.row as usize {
            let span = Span::styled(" ", Style::default().bg(Color::Blue));
            self.display.push_line(Line::from(span));
        } else {
            self.display.push_line(Line::from(Span::raw("")));
        }
    }

    fn process_wrapped_lines(
        &mut self,
        wrapped_lines: Vec<String>,
        current_row: usize,
        selection_bounds: &(usize, usize, usize, usize),
        added_characters: &mut usize,
    ) -> usize {
        let (start_row, start_col, end_row, end_col) = *selection_bounds;
        let mut local_row = current_row;

        for wrapped_line in wrapped_lines {
            let mut spans = Vec::new();
            let chars: Vec<char> = wrapped_line.chars().collect();

            // Track characters added for each line wrapped
            let original_line_length = chars.len();

            for (j, ch) in chars.into_iter().enumerate() {
                let should_select = self.cursor.should_select(
                    local_row, j, start_row, start_col, end_row, end_col,
                ) || (self.cursor.show_cursor()
                    && local_row == self.cursor.row as usize
                    && j == self.cursor.col as usize);

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

            self.display.push_line(Line::from(spans));
            local_row += 1;
        }

        local_row
    }

    fn update_cursor_style_in_insert_mode(&mut self) {
        if self.cursor.style() == WindowStyle::Insert {
            let row_index = self.cursor.row as usize;
            let line_length = self.display.get_max_col(self.cursor.row) as usize;  // Current line length
            let trailing_spaces = self.display.trailing_spaces;

            if let Some(current_line) = self.display.lines_mut().get_mut(row_index) {
                let cursor_position = self.cursor.col as usize;

                if cursor_position >= line_length {
                    // Cursor is at the end of the line
                    if trailing_spaces > 0 {
                        // Add trailing spaces back to the line (these were removed during wrapping)
                        let spaces = std::iter::repeat(' ').take(trailing_spaces).collect::<String>();
                        current_line.spans.push(Span::raw(spaces));
                    }

                    // Append one additional space for the cursor itself, and style it
                    current_line.spans.push(Span::styled(" ", Style::default().bg(Color::Yellow)));
                } else {
                    // Style the cursor's current position within the line
                    if cursor_position < current_line.spans.len() {
                        if let Some(span) = current_line.spans.get_mut(cursor_position) {
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
