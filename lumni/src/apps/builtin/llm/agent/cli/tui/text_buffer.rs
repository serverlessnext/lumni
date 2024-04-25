use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ScrollbarState;
use textwrap::{wrap, Options, WordSplitter};

use super::response_window::PromptRect;
use super::{Cursor, MoveCursor};


#[derive(Debug, Clone)]
pub struct TextBuffer<'a> {
    area: PromptRect,
    buffer_cache: String,       // incoming text buffer (collect stream parts here)
    buffer_processed: String,   // processed (finalized) text
    display_text: Vec<Line<'a>>,    // generated text used for display
    highlighted_text: String,   // highlighted text -- used for copying to clipboard
    cursor: Cursor,
    vertical_scroll: usize,
    vertical_scroll_state: ScrollbarState,
}

impl TextBuffer<'_> {
    pub fn new() -> Self {
        Self {
            area: PromptRect::default(),
            buffer_cache: String::new(),
            buffer_processed: String::new(),
            display_text: Vec::new(), 
            highlighted_text: String::new(),
            cursor: Cursor::new(0, 0),
            vertical_scroll: 0,
            vertical_scroll_state: ScrollbarState::default(),
        }
    }

    pub fn update_area(&mut self, area: &Rect) -> bool {
        self.area.update(area)
    }

    pub fn buffer_incoming(&self) -> &str {
        &self.buffer_cache
    }

    pub fn vertical_scroll_state(&mut self) -> &mut ScrollbarState {
        &mut self.vertical_scroll_state
    }

    pub fn push_incoming_text(&mut self, text: &str) {
        self.buffer_cache.push_str(text);
    }

    pub fn flush_incoming_buffer(&mut self) {
        // copy buffer to text
        self.buffer_processed.push_str(&self.buffer_cache);
        self.buffer_cache.clear();
    }

    pub fn display_text(&self) -> Vec<Line> {
        self.display_text.clone()
    }

    pub fn highlighted_text(&self) -> &str {
        // Return the highlighted text - e.g. for copying to clipboard
        &self.highlighted_text
    }

    pub fn vertical_scroll(&self) -> usize {
        self.vertical_scroll
    }

    pub fn set_vertical_scroll(&mut self) {

        //self.text_buffer.update_display_text();
        let length = self.content_length();
        let height = self.area.height() as usize;
        let scroll = if length > height {
            length - height
        } else {
            0
        };

        self.vertical_scroll = scroll;
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        let prev_col = self.cursor.col;
        let prev_row = self.cursor.row;

        let max_row = self.get_max_row();
        let next_row = match direction {
            MoveCursor::Up => self.cursor.row.saturating_sub(1),
            MoveCursor::Down => std::cmp::min(self.cursor.row + 1, max_row),
            _ => self.cursor.row,  // No change for left/right movements
        };

        self.cursor.move_cursor(
            direction.clone(),
            self.get_max_col(next_row as usize),
            max_row,
        );

        if self.cursor.show_cursor() {
            match direction {
                MoveCursor::Up => {
                    if self.cursor.row < self.vertical_scroll as u16 {
                        if self.vertical_scroll > 0 {
                            self.vertical_scroll -= 1; // Scroll up if not already at the top
                        }
                    }
                }
                MoveCursor::Down => {
                    let visible_rows = self.area.height() as usize;
                    if self.cursor.row
                        >= (self.vertical_scroll + visible_rows) as u16
                    {
                        let content_length = self.display_text.len();
                        if self.vertical_scroll < content_length - visible_rows
                        {
                            self.vertical_scroll += 1; // Scroll down if not already at the bottom
                        }
                    }
                }
                _ => {} // No scrolling necessary for left/right movement
            }

            // Re-update the display text to reflect the scroll change if necessary
            if prev_col != self.cursor.col || prev_row != self.cursor.row {
                self.update_display_text(); // Re-highlight cursor on new position
                self.update_scroll_state();
            }
        }
    }

    pub fn toggle_highlighting(&mut self) {
        self.cursor.toggle_highlighting();
        self.update_display_text();
    }

    pub fn set_highlighting(&mut self, enable: bool) {
        self.cursor.set_highlighting(enable);
        self.update_display_text();
    }

    fn get_max_col(&self, row: usize) -> u16 {
        // Get the current row where the cursor is located.
        if let Some(line) = self.display_text.get(row) {
            // Return the length of the line, considering all spans.
            line.spans
                .iter()
                .map(|span| span.content.len() as u16) // Calculate the length of each span
                .sum::<u16>()   // Sum up the lengths of all spans
                .saturating_sub(1) // Subtract 1 because the cursor is 0-indexed
        } else {
            0 // If for some reason the line doesn't exist, return 0
        }
    }

    fn get_max_row(&self) -> u16 {
        self.display_text.len() as u16 - 1
    }

    pub fn update_display_text(&mut self) {
        let display_width = self.area.width() as usize;
        let text = if self.buffer_cache.is_empty() {
            self.buffer_processed.clone()
        } else {
            format!("{}\n{}", self.buffer_processed, self.buffer_cache)
        };

        let mut new_display_text = Vec::new();
        self.highlighted_text.clear(); // Clear existing highlighted text
        let mut current_row = 0;

        // Determine the highlight bounds if highlighting is enabled
        let (start_row, start_col, end_row, end_col) = if self.cursor.is_highlighting_enabled() {
            self.cursor.get_highlight_bounds()
        } else {
            (usize::MAX, usize::MAX, usize::MIN, usize::MIN) // No highlighting
        };

        let mut line_has_content = false;

        for (_logical_row, line) in text.split('\n').enumerate() {
            let wrapped_lines = wrap(
                line,
                Options::new(display_width).word_splitter(WordSplitter::NoHyphenation),
            );

            if wrapped_lines.is_empty() {
                // Handle empty lines specifically
                if current_row == self.cursor.row as usize {
                    let spans = vec![Span::styled(" ", Style::default().bg(Color::Blue))];
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
                        let should_highlight = self.cursor.should_highlight(
                            current_row, j, start_row, start_col, end_row, end_col,
                        ) || (self.cursor.show_cursor() && current_row == self.cursor.row as usize && j == self.cursor.col as usize);

                        if should_highlight {
                            spans.push(Span::styled(
                                ch.to_string(),
                                Style::default().bg(Color::Blue),
                            ));
                            // Append highlighted character to the buffer
                            self.highlighted_text.push(ch); 
                        } else {
                            spans.push(Span::raw(ch.to_string()));
                        }
                    }
                    if spans.is_empty() && current_row == self.cursor.row as usize {
                        // Ensure cursor visibility on lines with no characters
                        spans.push(Span::styled(" ", Style::default().bg(Color::Blue)));
                    }
                    new_display_text.push(Line::from(spans));
                    current_row += 1;
                }
            }
        }
        if !line_has_content && current_row == self.cursor.row as usize {
            // This condition is specifically for the last empty line where the cursor might be
            let spans = vec![Span::styled(" ", Style::default().bg(Color::Blue))];
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
            self.update_scroll_state();
        } 
    }

    pub fn scroll_up(&mut self) {
        if self.vertical_scroll != 0 {
            self.vertical_scroll = self.vertical_scroll.saturating_sub(10);
            self.update_scroll_state();
        }
    }

    pub fn update_scroll_state(&mut self) {
        let display_length = self
            .display_text.len()
            .saturating_sub(self.area.height() as usize);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(display_length)
            .viewport_content_length(self.area.height().into())
            .position(self.vertical_scroll());
    }

    pub fn content_length(&self) -> usize {
        self.display_text.len()
    }
}
