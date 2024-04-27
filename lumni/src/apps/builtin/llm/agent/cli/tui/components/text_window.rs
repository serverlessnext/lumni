use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarState};

use super::cursor::MoveCursor;
use super::prompt_rect::PromptRect;
use super::{InsertMode, TextBuffer, WindowStyle, WindowType};

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

    pub fn get_window_style(&self) -> WindowStyle {
        self.window_type.style()
    }

    pub fn set_window_style(&mut self, style: WindowStyle) {
        self.window_type.set_style(style);
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
            .text_buffer
            .display_text_len()
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

pub trait TextWindowTrait<'a> {
    fn get_base(&mut self) -> &mut TextWindow<'a>;
    fn vertical_scroll_bar_state(&mut self) -> &mut ScrollbarState;
    fn widget(&mut self, area: &Rect) -> Paragraph;

    fn set_window_style(&mut self, style: WindowStyle) {
        self.get_base().set_window_style(style);
    }

    fn get_window_style(&mut self) -> WindowStyle {
        self.get_base().get_window_style()
    }

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

    fn text_buffer(&mut self) -> &mut TextBuffer<'a> {
        self.get_base().text_buffer()
    }

    fn is_active(&mut self) -> bool {
        self.get_window_style() != WindowStyle::InActive
    }

    fn set_style_normal(&mut self) {
        self.set_window_style(WindowStyle::Normal);
    }

    fn set_style_insert(&mut self) {
        self.set_window_style(WindowStyle::Insert);
    }

    fn set_style_visual(&mut self) {
        self.set_window_style(WindowStyle::Visual);
    }

    fn set_style_inactive(&mut self) {
        self.set_window_style(WindowStyle::InActive);
    }
    
    fn set_normal_mode(&mut self) {
        self.set_selection(false);
    }
}
