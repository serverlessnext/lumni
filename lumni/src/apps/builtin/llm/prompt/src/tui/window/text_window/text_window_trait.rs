use lumni::api::error::ApplicationError;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Paragraph, ScrollbarState};

use super::{
    CodeBlock, LineType, MoveCursor, TextBuffer, TextDocumentTrait, TextWindow,
    WindowContent, WindowKind, WindowStatus,
};
use crate::external as lumni;

pub trait TextWindowTrait<'a, T: TextDocumentTrait> {
    fn base(&mut self) -> &mut TextWindow<'a, T>;

    fn init(&mut self) {
        self.base().update_placeholder_text();
    }

    fn current_line_type(&mut self) -> Option<LineType> {
        self.base().current_line_type()
    }

    fn get_column_row(&mut self) -> (usize, usize) {
        self.base().get_column_row()
    }

    fn max_row_idx(&mut self) -> usize {
        self.base().max_row_idx()
    }

    fn current_code_block(&mut self) -> Option<CodeBlock> {
        self.base().current_code_block()
    }

    fn widget<'b>(&'b mut self, area: &Rect) -> Paragraph<'b>
    where
        'a: 'b,
        T: 'b,
    {
        let base = self.base();
        base.widget(area)
    }

    fn vertical_scroll_bar_state<'b>(&'b mut self) -> &'b mut ScrollbarState
    where
        'a: 'b,
        T: 'b,
    {
        self.base().scroller.vertical_scroll_bar_state()
    }

    fn set_window_status(&mut self, status: WindowStatus) {
        self.set_selection_anchor(false); // disable selection when changing status
        self.base().set_window_status(status);
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

    fn scroll_to_end(&mut self) {
        self.base().scroll_to_end();
    }

    fn move_cursor(&mut self, direction: MoveCursor) {
        self.base().move_cursor(direction);
    }

    fn text_insert_add(
        &mut self,
        text: &str,
        style: Option<Style>,
    ) -> Result<(), ApplicationError> {
        self.base().text_insert_add(text, style)
    }

    fn text_append(
        &mut self,
        text: &str,
        style: Option<Style>,
    ) -> Result<(), ApplicationError> {
        self.base().text_append(text, style)
    }

    fn text_append_with_insert(
        &mut self,
        text: &str,
        style: Option<Style>,
    ) -> Result<(), ApplicationError> {
        self.base().text_append_with_insert(text, style)
    }

    fn text_set(
        &mut self,
        text: &str,
        style: Option<Style>,
    ) -> Result<(), ApplicationError> {
        self.text_empty();
        self.text_insert_add(text, style)
    }

    fn text_delete_char(&mut self) -> Result<(), ApplicationError> {
        // single-char delete on cursor position
        self.base().text_delete(true, 1)
    }

    fn text_delete_backspace(&mut self) -> Result<(), ApplicationError> {
        // single-char backspace, move cursor to the left
        self.base().text_delete(false, 1)
    }

    fn set_selection_anchor(&mut self, enable: bool) {
        self.base().text_buffer.set_selection_anchor(enable);
    }

    fn text_buffer(&mut self) -> &mut TextBuffer<'a, T> {
        self.base().text_buffer()
    }

    fn text_undo(&mut self) -> Result<(), ApplicationError> {
        self.base().text_buffer.undo()
    }

    fn text_redo(&mut self) -> Result<(), ApplicationError> {
        self.base().text_buffer.redo()
    }

    fn text_empty(&mut self) {
        self.base().text_buffer.empty();
    }

    fn is_active(&mut self) -> bool {
        self.window_status() != WindowStatus::InActive
    }

    fn set_status_normal(&mut self) {
        let has_text = !self.text_buffer().is_empty();
        let window_content = if has_text {
            Some(WindowContent::Text)
        } else {
            None
        };
        self.set_window_status(WindowStatus::Normal(window_content))
    }

    fn set_status_visual(&mut self) {
        self.set_window_status(WindowStatus::Visual);
    }

    fn set_status_background(&mut self) {
        self.set_window_status(WindowStatus::Background);
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
