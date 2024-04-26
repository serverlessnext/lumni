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
    area: PromptRect,
    text_buffer: TextBuffer<'a>,
    window_type: WindowType,
    vertical_scroll: usize, // vertical scroll position (line index)
    vertical_scroll_bar_state: ScrollbarState, // visual state of the scrollbar
}

impl<'a> TextWindow<'a> {
    pub fn new(window_type: WindowType) -> Self {
        Self {
            area: PromptRect::default(),
            text_buffer: TextBuffer::new(),
            window_type,
            vertical_scroll: 0,
            vertical_scroll_bar_state: ScrollbarState::default(),
        }
    }
    
    pub fn vertical_scroll_bar_state(&mut self) -> &mut ScrollbarState {
        &mut self.vertical_scroll_bar_state
    }

    fn scroll_to_cursor(&mut self) {
        let (_, cursor_row) = self.text_buffer.cursor_position();
        let visible_rows = self.area.height();
        let scroll = if cursor_row >= visible_rows {
            cursor_row - visible_rows + 1
        } else {
            0
        };

        self.vertical_scroll = scroll as usize;
        self.update_scroll_bar();
    }

    pub fn scroll_down(&mut self) {
        let content_length = self.text_buffer.display_text_len();
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
            .text_buffer.display_text_len()
            .saturating_sub(self.area.height() as usize);
        self.vertical_scroll_bar_state = self
            .vertical_scroll_bar_state
            .content_length(display_length)
            .viewport_content_length(self.area.height().into())
            .position(self.vertical_scroll);
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        let (_, row_changed) = self.text_buffer.move_cursor(direction);
        if row_changed {
            self.scroll_to_cursor();
        }
    }

    pub fn text_buffer(&mut self) -> &mut TextBuffer<'a> {
        &mut self.text_buffer
    }

    pub fn widget(&mut self, area: &Rect) -> Paragraph {
        if self.area.update(area) == true {
            // re-fit text to updated display
            self.text_buffer.set_width(self.area.width() as usize);
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
            .scroll((self.vertical_scroll as u16, 0))
    }

    pub fn text_insert_create(&mut self, mode: InsertMode) {
        self.text_buffer.text_insert_create(mode);
    }

    pub fn text_insert_add(&mut self, text: &str) {
        self.text_buffer.text_insert_add(text);
        self.scroll_to_cursor();
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

    fn toggle_selection(&mut self) {
        self.get_base().text_buffer.toggle_selection();
    }

    fn set_selection(&mut self, enable: bool) {
        self.get_base().text_buffer.set_selection(enable);
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
        self.get_base().vertical_scroll_bar_state()
    }

    fn widget(&mut self, area: &Rect) -> Paragraph {
        self.get_base().widget(area)
    }

    fn set_normal_mode(&mut self) {
        self.set_selection(false);
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
