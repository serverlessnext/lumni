use std::error::Error;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarState};

use super::{ChatSession, MoveCursor, TextBuffer};

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

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
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
    text_buffer: TextBuffer<'a>,
    area: PromptRect,
    is_active: bool,
    vertical_scroll_state: ScrollbarState,
}

impl PromptLogWindow<'_> {
    pub fn new() -> Self {
        Self {
            chat_session: ChatSession::new(),
            text_buffer: TextBuffer::new(),
            area: PromptRect::default(),
            is_active: false,
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

    pub fn scroll_up(&mut self) {
        if self.text_buffer.scroll_up() {
            self.update_scroll_state();
        }
    }

    pub fn scroll_down(&mut self) {
        if self.text_buffer.scroll_down(&self.area) {
            self.update_scroll_state();
        }
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        self.text_buffer.move_cursor(direction, &self.area);
        // Update display or scroll state as needed here.
        self.update_display();
    }

    pub fn chat_session(&mut self) -> &mut ChatSession {
        &mut self.chat_session
    }

    pub fn vertical_scroll_state(&mut self) -> &mut ScrollbarState {
        &mut self.vertical_scroll_state
    }

    pub fn update_scroll_state(&mut self) {
        let display_length = self
            .text_buffer
            .content_length()
            .saturating_sub(self.area.height as usize);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(display_length)
            .viewport_content_length(self.area.height.into())
            .position(self.text_buffer.vertical_scroll());
    }

    pub fn widget(&mut self, area: &Rect) -> Paragraph {
        if self.area.update(area) == true {
            // re-fit text to updated display
            self.text_buffer.update_display_text(&self.area);
        }

        Paragraph::new(Text::from(self.text_buffer.display_text()))
            .block(
                Block::default()
                    .title(format!("active = {}", self.is_active))
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .scroll((self.text_buffer.vertical_scroll() as u16, 0))
    }

    pub fn update_display(&mut self) {
        self.text_buffer.update_display_text(&self.area);
        let length = self.text_buffer.content_length();
        let height = self.area.height as usize;
        self.text_buffer.set_vertical_scroll(if length > height {
            length - height
        } else {
            0
        });
        self.update_scroll_state();
    }

    pub fn buffer_incoming_append(&mut self, text: &str) {
        self.text_buffer.push_incoming_text(text);
        self.update_display();
    }

    pub fn buffer_incoming_flush(&mut self) {
        let answer = self.text_buffer.buffer_incoming().trim().to_string();

        self.text_buffer.flush_incoming_buffer();

        log::debug!("Buffer flushed: {}", answer);
        self.chat_session().update_last_exchange(answer);
        self.update_display();
    }
}
