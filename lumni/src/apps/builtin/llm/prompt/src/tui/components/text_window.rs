use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::block::Padding;
use ratatui::widgets::{Block, Paragraph, ScrollbarState};

use super::cursor::MoveCursor;
use super::rect_area::RectArea;
use super::scroller::Scroller;
use super::text_buffer::{CodeBlock, LineType};
use super::{TextBuffer, WindowConfig, WindowKind, WindowStatus};

#[derive(Debug, Clone)]
pub struct TextWindow<'a> {
    area: RectArea,
    window_type: WindowConfig,
    scroller: Scroller,
    text_buffer: TextBuffer<'a>,
}

impl<'a> TextWindow<'a> {
    pub fn new(window_type: WindowConfig) -> Self {
        let is_editable = window_type.is_editable();
        Self {
            area: RectArea::default(),
            window_type,
            scroller: Scroller::new(),
            text_buffer: TextBuffer::new(is_editable),
        }
    }

    pub fn window_status(&self) -> WindowStatus {
        self.window_type.window_status()
    }

    pub fn set_window_status(&mut self, status: WindowStatus) {
        if self.window_status() == status {
            return; // no change
        }

        match status {
            WindowStatus::Visual => {
                self.text_buffer.set_cursor_visibility(true);
            }
            WindowStatus::Insert => {
                self.text_buffer.set_cursor_visibility(true);
            }
            WindowStatus::Normal => {
                self.text_buffer.set_cursor_visibility(true);
            }
            WindowStatus::Background => {
                self.text_buffer.set_cursor_visibility(false);
            }
            _ => {
                self.text_buffer.set_cursor_visibility(false);
            }
        }
        // update window status if changed
        self.window_type.set_window_status(status);
        // update placeholder text
        let placeholder_text = self.window_type.placeholder_text();
        self.text_buffer.set_placeholder(placeholder_text);
    }

    fn scroll_to_cursor(&mut self) {
        let (_, cursor_row) = self.text_buffer.get_column_row(); // current cursor row
        let visible_rows = self.area.height() as usize; // max number of rows visible in the window

        let first_visible_row = self.scroller.vertical_scroll;
        let last_visible_row =
            first_visible_row.saturating_add(visible_rows.saturating_sub(1));

        // check if cursor is within visible area
        if cursor_row < first_visible_row {
            // cursor is above visible area
            self.scroller.vertical_scroll = cursor_row;
        } else if cursor_row > last_visible_row {
            // cursor is below visible area
            self.scroller.vertical_scroll =
                cursor_row.saturating_sub(visible_rows.saturating_sub(1));
        } else {
            // cursor is within visible area - no need to scroll
            return;
        }
        self.update_scroll_bar();
    }

    pub fn scroll_down(&mut self) {
        let content_length = self.text_buffer.display_lines_len();
        let area_height = self.area.height() as usize;
        let end_scroll = content_length.saturating_sub(area_height);
        if content_length > area_height {
            // scrolling enabled when content length exceeds area height
            if self.scroller.vertical_scroll + 10 <= end_scroll {
                self.scroller.vertical_scroll += 10;
            } else {
                self.scroller.vertical_scroll = end_scroll;
            }
            self.update_scroll_bar();
        }
    }

    pub fn scroll_to_end(&mut self) {
        self.text_buffer
            .move_cursor(MoveCursor::EndOfFileEndOfLine, false);

        let cursor_row = self.text_buffer.display_lines_len().saturating_sub(1);
        let visible_rows = self.area.height();
        self.scroller.vertical_scroll = if cursor_row >= visible_rows as usize {
            cursor_row - visible_rows as usize + 1
        } else {
            0
        };
        self.update_scroll_bar();
    }

    pub fn scroll_up(&mut self) {
        self.scroller.disable_auto_scroll(); // disable auto-scroll when manually scrolling

        if self.scroller.vertical_scroll != 0 {
            self.scroller.vertical_scroll =
                self.scroller.vertical_scroll.saturating_sub(10);
            self.update_scroll_bar();
        }
    }

    fn update_scroll_bar(&mut self) {
        let display_length = self
            .text_buffer
            .display_lines_len()
            .saturating_sub(self.area.height() as usize);
        self.scroller
            .update_scroll_bar(display_length, self.area.height() as usize);
    }

    pub fn move_cursor(&mut self, direction: MoveCursor) {
        self.scroller.disable_auto_scroll(); // disable auto-scroll when manually moving cursor
        let (_, row_changed) = self.text_buffer.move_cursor(direction, false);
        if row_changed {
            self.scroll_to_cursor();
        }
    }

    pub fn text_buffer(&mut self) -> &mut TextBuffer<'a> {
        &mut self.text_buffer
    }

    pub fn current_line_type(&self) -> Option<LineType> {
        let (_, row) = self.text_buffer.get_column_row();
        self.text_buffer.row_line_type(row)
    }

    pub fn current_code_block(&self) -> Option<CodeBlock> {
        let line_type = self.current_line_type();
        match line_type {
            Some(LineType::Code(code_block)) => {
                let ptr = code_block.get_ptr();
                if let Some(code_block) = self.text_buffer.get_code_block(ptr) {
                    return Some(code_block.clone());
                } else {
                    return None;
                }
            }
            _ => None,
        }
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
            .padding(Padding::new(0, 0, 0, 0));

        if let Some(title) = self.window_type.title() {
            block = block
                .title(title)
                .title_style(Style::default().fg(Color::LightGreen))
        }

        if let Some(hint) = self.window_type.hint() {
            block = block
                .title(hint)
        }

        let start_idx = self.scroller.vertical_scroll;
        let window_text = self.text_buffer.display_window_lines(
            start_idx,
            start_idx + self.area.height() as usize,
        );

        Paragraph::new(Text::from(window_text))
            .block(block)
            .style(self.window_type.style())
            .alignment(Alignment::Left)
    }

    pub fn text_insert_add(&mut self, text: &str, style: Option<Style>) {
        // inserted text is added at cursor position
        self.text_buffer.text_insert_add(text, style);
        self.scroll_to_cursor();
    }

    pub fn text_append_with_insert(
        &mut self,
        text: &str,
        style: Option<Style>,
    ) {
        // inserted text is appended at end of text
        if self.scroller.auto_scroll {
            self.scroll_to_end();
            self.text_buffer.text_insert_add(text, style);
        } else {
            self.text_buffer.text_append(text, style);
        }
    }

    pub fn text_delete(&mut self, include_cursor: bool, count: usize) {
        self.text_buffer.text_delete(include_cursor, count);
        self.scroll_to_cursor();
    }

    pub fn update_placeholder_text(&mut self) {
        let placeholder_text = self.window_type.placeholder_text();
        self.text_buffer.set_placeholder(placeholder_text);
    }
}

pub trait TextWindowTrait<'a> {
    fn base(&mut self) -> &mut TextWindow<'a>;

    fn init(&mut self) {
        self.base().update_placeholder_text();
    }

    fn current_line_type(&mut self) -> Option<LineType> {
        self.base().current_line_type()
    }

    fn current_code_block(&mut self) -> Option<CodeBlock> {
        self.base().current_code_block()
    }

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
        self.base().scroller.vertical_scroll_bar_state()
    }

    fn set_window_status(&mut self, status: WindowStatus) {
        self.set_selection_anchor(false); // disable selection when changing status
        self.base().set_window_status(status);
    }

    fn enable_auto_scroll(&mut self) {
        self.base().scroller.enable_auto_scroll();
    }

    fn is_editable(&mut self) -> bool {
        self.base().window_type.is_editable()
    }

    fn get_kind(&mut self) -> WindowKind {
        self.base().window_type.kind()
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

    fn text_set(&mut self, text: &str, style: Option<Style>) {
        self.text_empty();
        self.text_insert_add(text, style)
    }

    fn text_delete_char(&mut self) {
        // single-char delete on cursor position
        self.base().text_delete(true, 1);
    }

    fn text_delete_backspace(&mut self) {
        // single-char backspace, move cursor to the left
        self.base().text_delete(false, 1);
    }

    fn set_selection_anchor(&mut self, enable: bool) {
        self.base().text_buffer.set_selection_anchor(enable);
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

    fn is_active(&mut self) -> bool {
        self.window_status() != WindowStatus::InActive
    }

    fn set_status_normal(&mut self) {
        self.set_window_status(WindowStatus::Normal);
    }

    fn set_status_visual(&mut self) {
        self.set_window_status(WindowStatus::Visual);
    }

    fn set_status_background(&mut self) {
        self.set_window_status(WindowStatus::Background);
    }

    fn set_window_title(&mut self, title: &str) {
        self.base().window_type.set_title_text(title);
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
            self.base().text_buffer.set_selection_anchor(false);
            self.set_status_normal();
        } else {
            self.set_status_visual();
            self.base().text_buffer.set_selection_anchor(true);
        }
    }

    fn text_select_all(&mut self) {
        self.set_status_visual(); // enable visual mode
                                  // set selection anchor on start of text, move cursor to end of text
        self.base()
            .text_buffer
            .move_cursor(MoveCursor::StartOfFile, false);
        self.base().text_buffer.set_selection_anchor(true);
        self.base()
            .text_buffer
            .move_cursor(MoveCursor::EndOfFileEndOfLine, false);
    }

    fn text_unselect(&mut self) {
        self.base().text_buffer.set_selection_anchor(false);
        self.set_status_normal();
    }

    fn set_status_inactive(&mut self) {
        self.set_window_status(WindowStatus::InActive);
    }
}
