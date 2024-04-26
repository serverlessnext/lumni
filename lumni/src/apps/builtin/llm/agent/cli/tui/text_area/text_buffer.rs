use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ScrollbarState;
use textwrap::{wrap, Options, WordSplitter};

use super::piece_table::{InsertMode, PieceTable};
use super::{Cursor, MoveCursor, PromptRect};

#[derive(Debug, Clone)]
pub struct TextBuffer<'a> {
    area: PromptRect,
    text: PieceTable, // text buffer
    display_text: Vec<Line<'a>>, // text (e.g. wrapped,  highlighted) for display
    selected_text: String, // currently selected text
    cursor: Cursor,
    vertical_scroll: usize, // vertical scroll position (line index)
    vertical_scroll_bar_state: ScrollbarState, // visual state of the scrollbar
}

impl TextBuffer<'_> {
    pub fn new() -> Self {
        Self {
            area: PromptRect::default(),
            text: PieceTable::new(""),
            display_text: Vec::new(),
            selected_text: String::new(),
            cursor: Cursor::new(0, 0),
            vertical_scroll: 0,
            vertical_scroll_bar_state: ScrollbarState::default(),
        }
    }

    pub fn update_area(&mut self, area: &Rect) -> bool {
        self.area.update(area)
    }

    pub fn vertical_scroll_bar_state(&mut self) -> &mut ScrollbarState {
        &mut self.vertical_scroll_bar_state
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

    pub fn selected_text(&self) -> &str {
        // Return the highlighted text - e.g. for copying to clipboard
        &self.selected_text
    }

    pub fn vertical_scroll(&self) -> usize {
        self.vertical_scroll
    }

    fn scroll_to_cursor(&mut self) {
        let cursor_row = self.cursor.row as usize;
        let visible_rows = self.area.height() as usize;
        let scroll = if cursor_row >= visible_rows {
            cursor_row - visible_rows + 1
        } else {
            0
        };

        self.vertical_scroll = scroll;
        self.update_scroll_bar();
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        let prev_col = self.cursor.col;
        let prev_row = self.cursor.row;

        self.cursor.move_cursor(
            direction,
            &self.display_text, // pass display text to cursor for bounds checking
        );

        if self.cursor.show_cursor() {
            // Re-update the display text to reflect the scroll change if necessary
            if prev_col != self.cursor.col || prev_row != self.cursor.row {
                self.update_display_text(); // Re-highlight cursor on new position
                if prev_row != self.cursor.row {
                    self.scroll_to_cursor();
                }
            }
        }
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
        let display_width = self.area.width() as usize;
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
                Options::new(display_width)
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

    pub fn scroll_down(&mut self) {
        let content_length = self.display_text.len();
        let area_height = self.area.height() as usize;
        let end_scroll = content_length.saturating_sub(area_height);
        if content_length > area_height {
            // scrolling enabled when content length exceeds area height
            if self.vertical_scroll + 10 <= end_scroll {
                self.vertical_scroll += 10;
            } else {
                self.vertical_scroll = end_scroll;
            }
            self.update_scroll_bar();
        }
    }

    pub fn scroll_up(&mut self) {
        if self.vertical_scroll != 0 {
            self.vertical_scroll = self.vertical_scroll.saturating_sub(10);
            self.update_scroll_bar();
        }
    }

    fn update_scroll_bar(&mut self) {
        let display_length = self
            .display_text
            .len()
            .saturating_sub(self.area.height() as usize);
        self.vertical_scroll_bar_state = self
            .vertical_scroll_bar_state
            .content_length(display_length)
            .viewport_content_length(self.area.height().into())
            .position(self.vertical_scroll());
    }
}
