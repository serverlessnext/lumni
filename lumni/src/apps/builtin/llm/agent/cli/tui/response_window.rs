use std::error::Error;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarState};
use textwrap::{wrap, Options, WordSplitter};

use super::ChatSession;
use super::{Cursor, MoveCursor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl PromptRect {
    pub fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn update(&mut self, area: &Rect) -> bool {
        // adjust widget area for borders
        // return true if updated, else false
        let previous = *self; // copy current state

        self.x = area.x;
        self.y = area.y;
        self.width = area.width - 2;
        self.height = area.height - 2;

        if *self != previous {
            true
        } else {
            false
        }
    }
}

pub struct PromptLogWindow<'a> {
    chat_session: ChatSession,
    buffer_incoming: String, // incoming response buffer
    raw_text: String,        // text as received
    display_text: Vec<Line<'a>>, // text processed for display
    highlighted_text: String, // text with highlighted cursor
    area: PromptRect,
    is_active: bool,
    is_cursor_enabled: bool,
    cursor: Cursor,
    vertical_scroll: usize,
    vertical_scroll_state: ScrollbarState,
}

impl PromptLogWindow<'_> {
    pub fn new() -> Self {
        Self {
            chat_session: ChatSession::new(),
            buffer_incoming: String::new(),
            raw_text: String::new(),
            display_text: Vec::new(),
            highlighted_text: String::new(),
            area: PromptRect::default(),
            is_active: false,
            is_cursor_enabled: true,
            cursor: Cursor::new(0, 0),
            vertical_scroll: 0,
            vertical_scroll_state: ScrollbarState::default(),
        }
    }

    pub async fn init(&mut self) -> Result<(), Box<dyn Error>> {
        self.chat_session.init().await?;
        Ok(())
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn highlighted_text(&self) -> &str {
        &self.highlighted_text
    }

    pub fn chat_session(&mut self) -> &mut ChatSession {
        &mut self.chat_session
    }

    pub fn vertical_scroll_state(&mut self) -> &mut ScrollbarState {
        &mut self.vertical_scroll_state
    }

    pub fn scroll_down(&mut self) {
        let content_length = self.content_length();
        let area_height = self.area.height as usize;
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
        if self.vertical_scroll == 0 {
            return;
        }
        self.vertical_scroll = self.vertical_scroll.saturating_sub(10);
        self.update_scroll_state();
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        let prev_col = self.cursor.col;
        let prev_row = self.cursor.row;

        // Move the cursor based on the given direction
        self.cursor.move_cursor(direction.clone(), self.get_max_col(), self.get_max_row());

        // Check if the cursor's new position is outside the current view
        if self.is_cursor_enabled {
            match direction {
                MoveCursor::Up => {
                    if self.cursor.row < self.vertical_scroll as u16 {
                        if self.vertical_scroll > 0 {
                            self.vertical_scroll -= 1;  // Scroll up if not already at the top
                        }
                    }
                },
                MoveCursor::Down => {
                    let visible_rows = self.area.height as usize;
                    if self.cursor.row >= (self.vertical_scroll + visible_rows) as u16 {
                        let content_length = self.content_length();
                        if self.vertical_scroll < content_length - visible_rows {
                            self.vertical_scroll += 1;  // Scroll down if not already at the bottom
                        }
                    }
                },
                _ => {} // No scrolling necessary for left/right movement
            }

            // Re-update the display text to reflect the scroll change if necessary
            if (prev_col != self.cursor.col || prev_row != self.cursor.row) {
                self.update_display_text(); // Re-highlight cursor on new position
                self.update_scroll_state(); // Update the scroll state to apply the vertical scroll change
            }
        }
    }


    fn get_max_col(&self) -> u16 {
        // Get the current row where the cursor is located.
        if let Some(line) = self.display_text.get(self.cursor.row as usize) {
            // Return the length of the line, considering all spans.
            line.spans.iter()
                .map(|span| span.content.len() as u16) // Calculate the length of each span
                .sum() // Sum up the lengths of all spans
        } else {
            0 // If for some reason the line doesn't exist, return 0
        }
    }

    fn get_max_row(&self) -> u16 {
        self.display_text.len() as u16 - 1
    }

    fn update_display_text(&mut self) {
        let display_width = self.area.width as usize;
        let text = if self.buffer_incoming.is_empty() {
            self.raw_text.clone()
        } else {
            let combined_text = format!("{}\n{}", self.raw_text, self.buffer_incoming);
            combined_text
        };

        let mut new_display_text = Vec::new();
        self.highlighted_text.clear();  // Clear existing highlighted text
        let mut current_row = 0;

        let (start_row, start_col, end_row, end_col) = if self.cursor.row < self.cursor.fixed_row || 
            (self.cursor.row == self.cursor.fixed_row && self.cursor.col < self.cursor.fixed_col) {
            (self.cursor.row as usize, self.cursor.col as usize, self.cursor.fixed_row as usize, self.cursor.fixed_col as usize)
        } else {
            (self.cursor.fixed_row as usize, self.cursor.fixed_col as usize, self.cursor.row as usize, self.cursor.col as usize)
        };

        for (_logical_row, line) in text.split('\n').enumerate() {
            let wrapped_lines = wrap(
                line,
                Options::new(display_width).word_splitter(WordSplitter::NoHyphenation),
            );
            for wrapped_line in wrapped_lines {
                let mut spans = Vec::new();
                let chars: Vec<char> = wrapped_line.chars().collect();

                for (j, ch) in chars.into_iter().enumerate() {
                    let should_highlight = 
                        (current_row > start_row && current_row < end_row) ||
                        (current_row == start_row && current_row == end_row && j >= start_col && j <= end_col) ||
                        (current_row == start_row && j >= start_col && current_row < end_row) ||
                        (current_row == end_row && j <= end_col && current_row > start_row);

                    if should_highlight {
                        spans.push(Span::styled(ch.to_string(), Style::default().bg(Color::Blue)));
                        self.highlighted_text.push(ch);  // Append highlighted character to the buffer
                    } else {
                        spans.push(Span::raw(ch.to_string()));
                    }
                }
                new_display_text.push(Line::from(spans));
                current_row += 1;
            }
        }
        self.display_text = new_display_text;
    }


    pub fn update_scroll_state(&mut self) {
        let display_length = self
            .content_length()
            .saturating_sub(self.area.height as usize);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(display_length)
            .viewport_content_length(self.area.height.into())
            .position(self.vertical_scroll);
    }


    fn content_length(&self) -> usize {
        self.display_text.len()
    }

    pub fn widget(&mut self, area: &Rect) -> Paragraph {
        if self.area.update(area) == true {
            // re-fit text to updated display
            self.update_display_text();
        }

        Paragraph::new(Text::from(self.display_text.clone()))
            .block(Block::default().title(format!("active = {}", self.is_active)).borders(Borders::ALL))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .scroll((self.vertical_scroll as u16, 0))
    }

    pub fn update_display(&mut self) {
        self.update_display_text();
        let length = self.content_length();
        let height = self.area.height as usize;
        self.vertical_scroll =
            if length > height { length - height } else { 0 };
        self.update_scroll_state();
    }

    pub fn buffer_incoming_append(&mut self, text: &str) {
        self.buffer_incoming.push_str(text);
        self.update_display();
    }

    pub fn buffer_incoming_flush(&mut self) {
        let answer = self.buffer_incoming.clone().trim().to_string();
        self.buffer_incoming.clear();
        self.raw_text.push_str(&answer);
        log::debug!("Buffer flushed: {}", answer);
        self.chat_session().update_last_exchange(answer);
        self.update_display();
    }
}
