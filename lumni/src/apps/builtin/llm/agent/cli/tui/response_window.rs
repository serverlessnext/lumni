use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarState};

use super::{MoveCursor, TextBuffer, WindowKind, WindowStyle, WindowType};

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


pub struct TextWindow<'a> {
    text_buffer: TextBuffer<'a>,
    vertical_scroll_state: ScrollbarState,
    window_type: WindowType,
}

impl TextWindow<'_> {
    pub fn new(window_type: WindowType) -> Self {
        Self {
            text_buffer: TextBuffer::new(),
            vertical_scroll_state: ScrollbarState::default(),
            window_type,
        }
    }

    pub fn scroll_up(&mut self) {
        if self.text_buffer.scroll_up() {
            self.update_scroll_state();
        }
    }

    pub fn scroll_down(&mut self) {
        if self.text_buffer.scroll_down() {
            self.update_scroll_state();
        }
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        self.text_buffer.move_cursor(direction);
        // Update display or scroll state as needed here.
        self.update_display();
    }

    pub fn text_buffer(&mut self) -> &TextBuffer {
        &self.text_buffer
    }

    pub fn vertical_scroll_state(&mut self) -> &mut ScrollbarState {
        &mut self.vertical_scroll_state
    }

    pub fn update_scroll_state(&mut self) {
        let display_length = self
            .text_buffer
            .content_length()
            .saturating_sub(self.text_buffer.area().height as usize);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(display_length)
            .viewport_content_length(self.text_buffer.area().height.into())
            .position(self.text_buffer.vertical_scroll());
    }

    pub fn widget(&mut self, area: &Rect) -> Paragraph {
        if self.text_buffer.update_area(area) == true {
            // re-fit text to updated display
            self.text_buffer.update_display_text();
        }

        Paragraph::new(Text::from(self.text_buffer.display_text()))
            .block(
                Block::default()
                    .title(format!("{}", self.window_type.description()))
                    .borders(Borders::ALL)
                    .border_style(self.window_type.style().border_style()),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .scroll((self.text_buffer.vertical_scroll() as u16, 0))
    }

    pub fn update_display(&mut self) {
        self.text_buffer.update_display_text();
        let length = self.text_buffer.content_length();
        let height = self.text_buffer.area().height as usize;
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

    pub fn buffer_incoming_flush(&mut self) -> String {
        let text = self.text_buffer.buffer_incoming().trim().to_string();

        self.text_buffer.flush_incoming_buffer();
        text
    }
}

pub trait WindowTrait {
    fn text_buffer(&mut self) -> &TextBuffer;
    fn vertical_scroll_state(&mut self) -> &mut ScrollbarState;
    fn widget(&mut self, area: &Rect) -> Paragraph;
    fn set_normal_mode(&mut self);
}

pub trait TextWindowExt<'a> {
    fn get_base(&mut self) -> &mut TextWindow<'a>;

    fn scroll_up(&mut self) {
        self.get_base().scroll_up();
    }

    fn scroll_down(&mut self) {
        self.get_base().scroll_down();
    }

    fn move_cursor(&mut self, direction: MoveCursor) {
        self.get_base().move_cursor(direction);
    }

    fn buffer_incoming_append(&mut self, text: &str) {
        self.get_base().buffer_incoming_append(text);
    }

    fn buffer_incoming_flush(&mut self) -> String {
        self.get_base().buffer_incoming_flush()
    }

    fn toggle_highlighting(&mut self) {
        self.get_base().text_buffer.toggle_highlighting();
    }

    fn set_highlighting(&mut self, enable: bool) {
        self.get_base().text_buffer.set_highlighting(enable);
    }
}

pub struct ResponseWindow<'a> {
    base: TextWindow<'a>,
    is_active: bool,
}

impl WindowTrait for ResponseWindow<'_> {
    fn text_buffer(&mut self) -> &TextBuffer {
        self.base.text_buffer()
    }

    fn vertical_scroll_state(&mut self) -> &mut ScrollbarState {
        self.get_base().vertical_scroll_state()
    }

    fn widget(&mut self, area: &Rect) -> Paragraph {
        self.get_base().widget(area)
    }

    fn set_normal_mode(&mut self) {
        self.set_highlighting(false);
    }
}

impl ResponseWindow<'_> {
    pub fn new() -> Self {
        let window_type =
            WindowType::new(WindowKind::ResponseWindow, WindowStyle::InActive);
        Self {
            base: TextWindow::new(window_type),
            is_active: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn set_active(&mut self, active: bool) {
        // change style based on active state
        if active {
            self.base.window_type.set_style(WindowStyle::Normal);
        } else {
            self.base.window_type.set_style(WindowStyle::InActive);
        }
        self.is_active = active;
    }
}

impl<'a> TextWindowExt<'a> for ResponseWindow<'a> {
    fn get_base(&mut self) -> &mut TextWindow<'a> {
        &mut self.base
    }
}