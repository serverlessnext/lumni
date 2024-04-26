use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarState};

use super::{
    InsertMode, MoveCursor, TextBuffer, WindowKind, WindowStyle, WindowType,
};

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
    window_type: WindowType,
}

impl<'a> TextWindow<'a> {
    pub fn new(window_type: WindowType) -> Self {
        Self {
            text_buffer: TextBuffer::new(),
            window_type,
        }
    }

    pub fn scroll_up(&mut self) {
        self.text_buffer.scroll_up();
    }

    pub fn scroll_down(&mut self) {
        self.text_buffer.scroll_down();
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        self.text_buffer.move_cursor(direction);
    }

    pub fn text_buffer(&mut self) -> &mut TextBuffer<'a> {
        &mut self.text_buffer
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

    pub fn text_insert_create(&mut self, mode: InsertMode) {
        self.text_buffer.text_insert_create(mode);
    }

    pub fn text_insert_add(&mut self, text: &str) {
        self.text_buffer.text_insert_add(text);
    }

    pub fn text_insert_commit(&mut self) -> String {
        self.text_buffer.text_insert_commit()
    }
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

    fn text_insert_create(&mut self, mode: InsertMode) {
        self.get_base().text_insert_create(mode);
    }

    fn text_insert_add(&mut self, text: &str) {
        self.get_base().text_insert_add(text);
    }

    fn text_insert_commit(&mut self) -> String {
        self.get_base().text_insert_commit()
    }

    fn toggle_highlighting(&mut self) {
        self.get_base().text_buffer.toggle_highlighting();
    }

    fn set_highlighting(&mut self, enable: bool) {
        self.get_base().text_buffer.set_highlighting(enable);
    }
}

pub trait TextWindowTrait<'a> {
    fn text_buffer(&mut self) -> &mut TextBuffer<'a>;
    fn vertical_scroll_bar_state(&mut self) -> &mut ScrollbarState;
    fn widget(&mut self, area: &Rect) -> Paragraph;
    fn set_normal_mode(&mut self);
}

pub struct ResponseWindow<'a> {
    base: TextWindow<'a>,
    is_active: bool,
}

impl<'a> TextWindowTrait<'a> for ResponseWindow<'a> {
    fn text_buffer(&mut self) -> &mut TextBuffer<'a> {
        self.get_base().text_buffer()
    }

    fn vertical_scroll_bar_state(&mut self) -> &mut ScrollbarState {
        self.text_buffer().vertical_scroll_bar_state()
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
