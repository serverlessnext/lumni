use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use textwrap::{wrap, Options, WordSplitter};

use super::response_window::PromptRect;
use super::{Cursor, MoveCursor};

#[derive(Debug, Clone)]
pub struct TextBuffer<'a> {
    area: PromptRect,
    buffer_incoming: String, // incoming response buffer
    raw_text: String,
    display_text: Vec<Line<'a>>,
    highlighted_text: String,
    cursor: Cursor,
    is_cursor_enabled: bool,
    vertical_scroll: usize,
}

impl TextBuffer<'_> {
    pub fn new() -> Self {
        Self {
            area: PromptRect::default(),
            buffer_incoming: String::new(),
            raw_text: String::new(),
            display_text: Vec::new(),
            highlighted_text: String::new(),
            cursor: Cursor::new(0, 0),
            is_cursor_enabled: true,
            vertical_scroll: 0,
        }
    }

    pub fn area(&self) -> &PromptRect {
        &self.area
    }

    pub fn update_area(&mut self, area: &Rect) -> bool {
        self.area.update(area)
    }

    pub fn buffer_incoming(&self) -> &str {
        &self.buffer_incoming
    }

    pub fn push_incoming_text(&mut self, text: &str) {
        self.buffer_incoming.push_str(text);
    }

    pub fn flush_incoming_buffer(&mut self) {
        // copy buffer to text
        self.raw_text.push_str(&self.buffer_incoming);
        self.buffer_incoming.clear();
    }

    pub fn display_text(&self) -> Vec<Line> {
        self.display_text.clone()
    }

    pub fn highlighted_text(&self) -> &str {
        &self.highlighted_text
    }

    pub fn vertical_scroll(&self) -> usize {
        self.vertical_scroll
    }

    pub fn set_vertical_scroll(&mut self, scroll: usize) {
        self.vertical_scroll = scroll;
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        let prev_col = self.cursor.col;
        let prev_row = self.cursor.row;

        self.cursor.move_cursor(
            direction.clone(),
            self.get_max_col(),
            self.get_max_row(),
        );

        if self.is_cursor_enabled {
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
                        let content_length = self.content_length();
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
            }
        }
    }

    pub fn toggle_highlighting(&mut self) {
        self.cursor.toggle_highlighting();
        self.update_display_text();
    }

    fn get_max_col(&self) -> u16 {
        // Get the current row where the cursor is located.
        if let Some(line) = self.display_text.get(self.cursor.row as usize) {
            // Return the length of the line, considering all spans.
            line.spans
                .iter()
                .map(|span| span.content.len() as u16) // Calculate the length of each span
                .sum() // Sum up the lengths of all spans
        } else {
            0 // If for some reason the line doesn't exist, return 0
        }
    }

    fn get_max_row(&self) -> u16 {
        self.display_text.len() as u16 - 1
    }

    pub fn update_display_text(&mut self) {
        let display_width = self.area.width() as usize;
        let text = if self.buffer_incoming.is_empty() {
            self.raw_text.clone()
        } else {
            format!("{}\n{}", self.raw_text, self.buffer_incoming)
        };

        let mut new_display_text = Vec::new();
        self.highlighted_text.clear(); // Clear existing highlighted text
        let mut current_row = 0;

        // Only apply highlighting logic if highlighting is enabled
        if self.cursor.is_highlighting_enabled {
            let (start_row, start_col, end_row, end_col) =
                self.cursor.get_highlight_bounds();

            for (_logical_row, line) in text.split('\n').enumerate() {
                let wrapped_lines = wrap(
                    line,
                    Options::new(display_width)
                        .word_splitter(WordSplitter::NoHyphenation),
                );
                for wrapped_line in wrapped_lines {
                    let mut spans = Vec::new();
                    let chars: Vec<char> = wrapped_line.chars().collect();

                    for (j, ch) in chars.into_iter().enumerate() {
                        let should_highlight = self.cursor.should_highlight(
                            current_row,
                            j,
                            start_row,
                            start_col,
                            end_row,
                            end_col,
                        );

                        if should_highlight {
                            spans.push(Span::styled(
                                ch.to_string(),
                                Style::default().bg(Color::Blue),
                            ));
                            self.highlighted_text.push(ch);
                        } else {
                            spans.push(Span::raw(ch.to_string()));
                        }
                    }
                    new_display_text.push(Line::from(spans));
                    current_row += 1;
                }
            }
        } else {
            // Handle non-highlighted text generation
            for (_logical_row, line) in text.split('\n').enumerate() {
                let wrapped_lines = wrap(
                    line,
                    Options::new(display_width)
                        .word_splitter(WordSplitter::NoHyphenation),
                );
                for wrapped_line in wrapped_lines {
                    new_display_text
                        .push(Line::from(Span::raw(wrapped_line.to_string())));
                    current_row += 1;
                }
            }
        }
        self.display_text = new_display_text;
    }

    pub fn scroll_down(&mut self) -> bool {
        let content_length = self.content_length();
        let area_height = self.area.height() as usize;
        let end_scroll = content_length.saturating_sub(area_height);
        if content_length > area_height {
            // scrolling enabled when content length exceeds area height
            if self.vertical_scroll + 10 <= end_scroll {
                self.vertical_scroll += 10;
            } else {
                self.vertical_scroll = end_scroll;
            }
            true
        } else {
            false
        }
    }

    pub fn scroll_up(&mut self) -> bool {
        if self.vertical_scroll == 0 {
            return false;
        }
        self.vertical_scroll = self.vertical_scroll.saturating_sub(10);
        //self.update_scroll_state();
        true
    }
    pub fn content_length(&self) -> usize {
        self.display_text.len()
    }
}
