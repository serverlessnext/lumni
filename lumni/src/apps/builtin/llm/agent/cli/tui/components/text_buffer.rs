use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use textwrap::{wrap, Options, WordSplitter};

use super::cursor::{Cursor, MoveCursor};
use super::piece_table::{InsertMode, PieceTable};
use super::window_type::WindowStyle;

#[derive(Debug, Clone)]
pub struct TextBuffer<'a> {
    text: PieceTable,            // text buffer
    display_text: Vec<Line<'a>>, // text (e.g. wrapped,  highlighted) for display
    display_width: usize,        // width of the display area
    selected_text: String,       // currently selected text
    cursor: Cursor,
}

impl TextBuffer<'_> {
    pub fn new() -> Self {
        Self {
            text: PieceTable::new(""),
            display_text: Vec::new(),
            display_width: 0,
            selected_text: String::new(),
            cursor: Cursor::new(0, 0),
        }
    }

    pub fn empty(&mut self) {
        self.display_text.clear();
        self.selected_text.clear();
        self.cursor.reset();
        self.text.empty();
        // update display
        self.update_display_text();
    }

    pub fn set_width(&mut self, width: usize) {
        self.display_width = width;
    }

    pub fn set_cursor_style(&mut self, style: WindowStyle) {
        self.cursor.set_style(style);
    }

    pub fn text_insert_add(&mut self, text: &str) {
        self.text.cache_insert(text);
        self.update_display_text();
        self.move_cursor(MoveCursor::EndOfFileEndOfLine);
    }

    pub fn text_delete(&mut self, include_cursor: bool, _count: usize) {
        // ignore count for now -- only delete one character
        let idx = self.cursor.real_position();
        if include_cursor {
            self.text.delete(idx, 1);
        } else {
            if idx > 0 {
                self.text.delete(idx - 1, 1);
            } else {
                return;
            }
        }
        // move cursor to the left on successful delete
        let (column_changed, row_changed) = self.move_cursor(MoveCursor::Left);
        if !column_changed && !row_changed {
            // move_cursor only updates on column or row change
            // so if we are at the beginning of the line,
            // we need to update the display here
            self.update_display_text();
        }
    }

    pub fn text_insert_commit(&mut self) -> String {
        self.text.commit_insert_cache()
    }

    pub fn display_text(&self) -> Vec<Line> {
        self.display_text.clone()
    }

    pub fn display_text_len(&self) -> usize {
        self.display_text.len()
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
            &self.display_text, // pass display text to cursor for bounds checking
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
        self.display_text.clear();
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
        self.cursor
            .update_real_position(&self.display_text, added_characters);
        self.update_cursor_style_in_insert_mode(current_row);
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
            Options::new(self.display_width)
                .word_splitter(WordSplitter::NoHyphenation),
        )
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect()
    }

    fn handle_empty_line(&mut self, current_row: usize) {
        if current_row == self.cursor.row as usize {
            let span = Span::styled(" ", Style::default().bg(Color::Blue));
            self.display_text.push(Line::from(span));
        } else {
            self.display_text.push(Line::from(Span::raw("")));
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

            self.display_text.push(Line::from(spans));
            local_row += 1;
        }

        local_row
    }

    fn update_cursor_style_in_insert_mode(&mut self, current_row: usize) {
        if self.cursor.style() == WindowStyle::Insert
            && self.cursor.row as usize == current_row - 1
        {
            if let Some(last_line) = self.display_text.last_mut() {
                last_line.spans.push(Span::raw(" "));
                let line_length = last_line
                    .spans
                    .iter()
                    .map(|span| span.content.len())
                    .sum::<usize>();
                if (self.cursor.col as usize) < (line_length - 1) {
                    last_line.spans[self.cursor.col as usize].style =
                        Style::default().bg(Color::Yellow);
                } else {
                    if let Some(last_span) = last_line.spans.last_mut() {
                        last_span.style = Style::default().bg(Color::Yellow);
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
