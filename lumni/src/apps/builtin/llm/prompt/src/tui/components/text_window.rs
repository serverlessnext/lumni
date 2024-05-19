use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::block::Padding;
use ratatui::widgets::{Block, Paragraph, ScrollbarState};

use super::cursor::MoveCursor;
use super::prompt_rect::PromptRect;
use super::window_type::Highlighted;
use super::{TextBuffer, WindowStatus, WindowType};

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
            text_buffer: TextBuffer::new(window_type.is_editable()),
            window_type,
            vertical_scroll: 0,
            vertical_scroll_bar_state: ScrollbarState::default(),
        }
    }

    pub fn window_type(&self) -> WindowType {
        self.window_type
    }

    pub fn window_status(&self) -> WindowStatus {
        self.window_type.window_status()
    }

    pub fn set_window_status(&mut self, status: WindowStatus) {
        // enable cursor visibility based on window status
        match status {
            WindowStatus::Visual => {
                self.text_buffer.set_cursor_visibility(true);
            }
            WindowStatus::Insert => {
                self.text_buffer.set_cursor_visibility(true);
            }
            WindowStatus::Normal(highlighted) => {
                if highlighted == Highlighted::True {
                    self.text_buffer.set_cursor_visibility(true);
                } else {
                    self.text_buffer.set_cursor_visibility(false);
                }
            }
            _ => {
                self.text_buffer.set_cursor_visibility(false);
            }
        }
        self.window_type.set_window_status(status);
    }

    fn scroll_to_cursor(&mut self) {
        let (_, cursor_row) = self.text_buffer.display_column_row();
        let visible_rows = self.area.height();
        let scroll = if cursor_row >= visible_rows as usize {
            cursor_row - visible_rows as usize + 1
        } else {
            0
        };

        self.vertical_scroll = scroll;
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

    pub fn scroll_to_end(&mut self) {
        self.text_buffer
            .move_cursor(MoveCursor::EndOfFileEndOfLine, false);
        self.scroll_to_cursor();
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
        let (_, row_changed) = self.text_buffer.move_cursor(direction, false);
        if row_changed {
            self.scroll_to_cursor();
        }
    }

    pub fn text_buffer(&mut self) -> &mut TextBuffer<'a> {
        &mut self.text_buffer
    }

    pub fn widget<'b>(&'b mut self, area: &Rect) -> Paragraph<'b> {
        if self.area.update(area) == true {
            // re-fit text to updated display
            self.text_buffer.set_width(self.area.width() as usize);
            self.text_buffer.update_display_text();
        }

        let mut block = Block::default()
            .borders(self.window_type.borders())
            .border_style(self.window_type.border_style())
            .padding(Padding::new(1, 1, 0, 0));

        let description = format!("{}", self.window_type.description());
        if !description.is_empty() {
            block = block.title(description);
        }

        Paragraph::new(Text::from(self.text_buffer.display_text()))
            .block(block)
            .style(self.window_type.style())
            .alignment(Alignment::Left)
            .scroll((self.vertical_scroll as u16, 0))
    }

    pub fn vertical_scroll_bar_state<'b>(
        &'b mut self,
    ) -> &'b mut ScrollbarState {
        &mut self.vertical_scroll_bar_state
    }

    pub fn text_insert_add(&mut self, text: &str, style: Option<Style>) {
        self.text_buffer.text_insert_add(text, style);
        self.scroll_to_cursor();
    }

    pub fn text_append_with_insert(
        &mut self,
        text: &str,
        style: Option<Style>,
    ) {
        // inserted text is appended at end of text
        self.scroll_to_end();
        self.text_buffer.text_insert_add(text, style);
    }

    pub fn text_insert_commit(&mut self) -> String {
        self.text_buffer.text_insert_commit()
    }

    pub fn text_append(&mut self, text: &str, style: Option<Style>) {
        self.text_buffer.text_append(text, style);
        self.scroll_to_end();
    }

    pub fn text_delete(&mut self, include_cursor: bool, count: usize) {
        self.text_buffer.text_delete(include_cursor, count);
        self.scroll_to_cursor();
    }
}

pub trait TextWindowTrait<'a> {
    fn base(&mut self) -> &mut TextWindow<'a>;

    fn widget<'b>(&'b mut self, area: &Rect) -> Paragraph<'b>
    where
        'a: 'b,
    {
        let base = self.base();
        base.widget(area)
    }

    fn vertical_scroll_bar_state<'b>(&'b mut self) -> &'b mut ScrollbarState
    where
        'a: 'b,
    {
        self.base().vertical_scroll_bar_state()
    }

    fn set_window_status(&mut self, status: WindowStatus) {
        self.set_selection(false); // disable selection when changing status
        self.base().set_window_status(status);
    }

    fn window_type(&mut self) -> WindowType {
        self.base().window_type()
    }

    fn window_status(&mut self) -> WindowStatus {
        self.base().window_status()
    }

    fn scroll_up(&mut self) {
        self.base().scroll_up();
    }

    fn scroll_down(&mut self) {
        self.base().scroll_down();
    }

    fn move_cursor(&mut self, direction: MoveCursor) {
        self.base().move_cursor(direction);
    }

    fn text_insert_add(&mut self, text: &str, style: Option<Style>) {
        self.base().text_insert_add(text, style);
    }

    fn text_append_with_insert(&mut self, text: &str, style: Option<Style>) {
        self.base().text_append_with_insert(text, style);
    }

    fn text_insert_commit(&mut self) -> String {
        self.base().text_insert_commit()
    }

    fn text_append(&mut self, text: &str, style: Option<Style>) {
        self.base().text_append(text, style);
    }

    fn text_set(&mut self, text: &str, style: Option<Style>) {
        self.text_empty();
        self.text_append(text, style);
    }

    fn text_delete_char(&mut self) {
        // single-char delete on cursor position
        self.base().text_delete(true, 1);
    }

    fn text_delete_backspace(&mut self) {
        // single-char backspace, move cursor to the left
        self.base().text_delete(false, 1);
    }

    fn set_selection(&mut self, enable: bool) {
        self.base().text_buffer.set_selection(enable);
    }

    fn text_buffer(&mut self) -> &mut TextBuffer<'a> {
        self.base().text_buffer()
    }

    fn text_undo(&mut self) {
        self.base().text_buffer.undo();
    }

    fn text_redo(&mut self) {
        self.base().text_buffer.redo();
    }

    fn text_empty(&mut self) {
        self.base().text_buffer.empty();
    }

    fn text_set_placeholder(&mut self, text: &str) {
        self.base().text_buffer.set_placeholder(text);
    }

    fn is_active(&mut self) -> bool {
        self.window_status() != WindowStatus::InActive
    }

    fn set_status_normal(&mut self) {
        self.set_window_status(WindowStatus::Normal(Highlighted::True));
    }

    fn set_status_visual(&mut self) {
        self.set_window_status(WindowStatus::Visual);
    }

    fn set_status_background(&mut self) {
        self.set_window_status(WindowStatus::Normal(Highlighted::False));
    }

    fn is_status_insert(&mut self) -> bool {
        self.window_status() == WindowStatus::Insert
    }

    fn set_status_insert(&mut self) {
        self.set_window_status(WindowStatus::Insert);
    }

    fn toggle_visual_mode(&mut self) {
        // if visual mode is enabled, set to normal mode
        if self.window_status() == WindowStatus::Visual {
            self.base().text_buffer.set_selection(false);
            self.set_status_normal();
        } else {
            self.set_status_visual();
            self.base().text_buffer.set_selection(true);
        }
    }

    fn set_status_inactive(&mut self) {
        self.set_window_status(WindowStatus::InActive);
    }

    fn set_normal_mode(&mut self) {
        self.set_status_normal();
    }

    fn set_insert_mode(&mut self) {
        self.set_status_insert();
    }
}