use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use textwrap::{wrap, Options, WordSplitter};

use super::cursor::{Cursor, MoveCursor};
use super::piece_table::{InsertMode, PieceTable};

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

    pub fn set_width(&mut self, width: usize) {
        self.display_width = width;
    }

    pub fn text_insert_create(&mut self, mode: InsertMode) {
        self.text.start_insert_cache(mode);
    }

    pub fn text_insert_add(&mut self, text: &str) {
        self.text.cache_insert(text);
        self.update_display_text();
        self.move_cursor(MoveCursor::EndOfFileEndOfLine);
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

    pub fn toggle_selection(&mut self) {
        self.cursor.toggle_selection();
        self.update_display_text();
    }

    pub fn set_selection(&mut self, enable: bool) {
        self.cursor.set_selection(enable);
        self.update_display_text();
    }

    pub fn update_display_text(&mut self) {
        let text = self.text.content();

        let mut new_display_text = Vec::new();
        self.selected_text.clear(); // clear text selection
        let mut current_row = 0;

        // Determine the highlight bounds if highlighting is enabled
        let (start_row, start_col, end_row, end_col) =
            if self.cursor.selection_enabled() {
                self.cursor.get_selection_bounds()
            } else {
                (usize::MAX, usize::MAX, usize::MIN, usize::MIN) // No highlighting
            };

        let mut line_has_content = false;

        for (_logical_row, line) in text.split('\n').enumerate() {
            let wrapped_lines = wrap(
                line,
                Options::new(self.display_width)
                    .word_splitter(WordSplitter::NoHyphenation),
            );

            if wrapped_lines.is_empty() {
                // Handle empty lines specifically
                if current_row == self.cursor.row as usize {
                    let spans = vec![Span::styled(
                        " ",
                        Style::default().bg(Color::Blue),
                    )];
                    new_display_text.push(Line::from(spans));
                    line_has_content = true;
                } else {
                    new_display_text.push(Line::from(Span::raw("")));
                }
            } else {
                for wrapped_line in wrapped_lines {
                    let mut spans = Vec::new();
                    let chars: Vec<char> = wrapped_line.chars().collect();

                    for (j, ch) in chars.into_iter().enumerate() {
                        let should_select = self.cursor.should_select(
                            current_row,
                            j,
                            start_row,
                            start_col,
                            end_row,
                            end_col,
                        ) || (self.cursor.show_cursor()
                            && current_row == self.cursor.row as usize
                            && j == self.cursor.col as usize);

                        if should_select {
                            spans.push(Span::styled(
                                ch.to_string(),
                                Style::default().bg(Color::Blue),
                            ));
                            // Append highlighted character to the buffer
                            self.selected_text.push(ch);
                        } else {
                            spans.push(Span::raw(ch.to_string()));
                        }
                    }
                    if spans.is_empty()
                        && current_row == self.cursor.row as usize
                    {
                        // Ensure cursor visibility on lines with no characters
                        spans.push(Span::styled(
                            " ",
                            Style::default().bg(Color::Blue),
                        ));
                    }
                    new_display_text.push(Line::from(spans));
                    current_row += 1;
                }
            }
        }
        if !line_has_content && current_row == self.cursor.row as usize {
            // This condition is specifically for the last empty line where the cursor might be
            let spans =
                vec![Span::styled(" ", Style::default().bg(Color::Blue))];
            new_display_text.push(Line::from(spans));
        }
        self.display_text = new_display_text;
    }
}
