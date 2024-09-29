use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;
use ratatui::widgets::{
    Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
    StatefulWidgetRef, Widget,
};
use ratatui::Frame;

use super::{
    KeyTrack, MoveCursor, ReadDocument, ReadWriteDocument, TextBuffer,
    TextDocumentTrait, TextLine,
};

#[derive(Debug, Clone)]
pub struct TextArea<T: TextDocumentTrait> {
    widget: TextAreaWidget<T>,
    state: TextAreaState<'static, T>,
}

impl<T: TextDocumentTrait> TextArea<T> {
    pub fn new() -> Self
    where
        T: Default,
    {
        Self {
            widget: TextAreaWidget::new(),
            state: TextAreaState::new(T::default()),
        }
    }

    pub fn with_state(state: TextAreaState<'static, T>) -> Self {
        Self {
            widget: TextAreaWidget::new(),
            state,
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        self.state.handle_key_event(key_event);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        f.render_stateful_widget(&self.widget, area, &mut self.state);
    }
}

impl TextArea<ReadDocument> {
    pub fn with_read_document(text: Option<Vec<TextLine>>) -> Self {
        Self::with_state(TextAreaState::with_read_document(text))
    }
}

impl TextArea<ReadWriteDocument> {
    pub fn with_read_write_document(text: Option<Vec<TextLine>>) -> Self {
        Self::with_state(TextAreaState::with_read_write_document(text))
    }
}
#[derive(Debug, Clone)]
pub struct TextAreaWidget<T: TextDocumentTrait>(std::marker::PhantomData<T>);

#[derive(Debug, Clone)]
pub struct TextAreaState<'a, T: TextDocumentTrait> {
    text_buffer: TextBuffer<'a, T>,
    key_track: KeyTrack,
    scroll_offset: usize,
    viewport_height: usize,
}

impl<T: TextDocumentTrait> TextAreaWidget<T> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }

    fn render_scrollbar(
        &self,
        buf: &mut Buffer,
        area: Rect,
        state: &TextAreaState<'_, T>,
    ) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        let total_lines = state.text_buffer.display_lines_len();
        let viewport_height = state.viewport_height;

        let scrollable_area =
            total_lines.saturating_sub(viewport_height).max(1);

        let position = (state.scroll_offset as f64 / scrollable_area as f64
            * scrollable_area as f64)
            .round() as usize;

        let mut scrollbar_state =
            ScrollbarState::new(scrollable_area).position(position);

        StatefulWidget::render(
            scrollbar,
            scrollbar_area,
            buf,
            &mut scrollbar_state,
        );
    }
}

impl<'a, T: TextDocumentTrait> TextAreaState<'a, T> {
    pub fn new(document: T) -> Self {
        Self {
            text_buffer: TextBuffer::new(document),
            key_track: KeyTrack::new(),
            scroll_offset: 0,
            viewport_height: 0,
        }
    }

    pub fn text_buffer(&self) -> &TextBuffer<'a, T> {
        &self.text_buffer
    }

    pub fn text_buffer_mut(&mut self) -> &mut TextBuffer<'a, T> {
        &mut self.text_buffer
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        self.key_track.process_key(key_event);

        match key_event.code {
            KeyCode::Up => self.move_cursor(MoveCursor::Up(1)),
            KeyCode::Down => self.move_cursor(MoveCursor::Down(1)),
            KeyCode::Left => self.move_cursor(MoveCursor::Left(1)),
            KeyCode::Right => self.move_cursor(MoveCursor::Right(1)),
            KeyCode::Home => self.move_cursor(MoveCursor::StartOfLine),
            KeyCode::End => self.move_cursor(MoveCursor::EndOfLine),
            KeyCode::PageUp => self.scroll_up(self.viewport_height),
            KeyCode::PageDown => self.scroll_down(self.viewport_height),
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Delete => self.handle_delete(),
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Char(c) => self.handle_char(c),
            KeyCode::Tab => self.handle_tab(),
            _ => {
                // Handle other keys or modifiers if needed
            }
        }

        self.update_scroll();
    }

    fn move_cursor(&mut self, direction: MoveCursor) {
        self.text_buffer.move_cursor(direction, true);
    }

    fn handle_backspace(&mut self) {
        if let Err(e) = self.text_buffer.text_delete(false, 1) {
            eprintln!("Error handling backspace: {:?}", e);
        }
    }

    fn handle_delete(&mut self) {
        if let Err(e) = self.text_buffer.text_delete(true, 1) {
            eprintln!("Error handling delete: {:?}", e);
        }
    }

    fn handle_enter(&mut self) {
        if let Err(e) = self.text_buffer.text_insert_add("\n", None) {
            eprintln!("Error handling enter: {:?}", e);
        }
    }

    fn handle_char(&mut self, c: char) {
        if let Err(e) = self.text_buffer.text_insert_add(&c.to_string(), None) {
            eprintln!("Error handling character input: {:?}", e);
        }
    }

    fn handle_tab(&mut self) {
        if let Err(e) = self.text_buffer.text_insert_add("    ", None) {
            eprintln!("Error handling tab: {:?}", e);
        }
    }

    fn update_scroll(&mut self) {
        let (_, cursor_row) = self.text_buffer.get_column_row();
        if cursor_row < self.scroll_offset {
            self.scroll_offset = cursor_row;
        } else if cursor_row >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = cursor_row - self.viewport_height + 1;
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: usize) {
        let total_lines = self.text_buffer.display_lines_len();
        let max_scroll = total_lines.saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }
}

impl<'a> TextAreaState<'a, ReadDocument> {
    pub fn with_read_document(text: Option<Vec<TextLine>>) -> Self {
        let document = if let Some(text) = text {
            ReadDocument::from_text(text)
        } else {
            ReadDocument::new()
        };
        Self::new(document)
    }
}

impl<'a> TextAreaState<'a, ReadWriteDocument> {
    pub fn with_read_write_document(text: Option<Vec<TextLine>>) -> Self {
        let document = if let Some(text) = text {
            ReadWriteDocument::from_text(text)
        } else {
            ReadWriteDocument::new()
        };
        Self::new(document)
    }
}

impl<T: TextDocumentTrait> StatefulWidget for &TextAreaWidget<T> {
    type State = TextAreaState<'static, T>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidgetRef::render_ref(&self, area, buf, state)
    }
}

impl<T: TextDocumentTrait> StatefulWidgetRef for &TextAreaWidget<T> {
    type State = TextAreaState<'static, T>;

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State,
    ) {
        state.text_buffer.set_width(area.width as usize);
        state.text_buffer.update_display_text();

        let total_lines = state.text_buffer.display_lines_len();
        let viewport_height = area.height as usize;

        state.set_viewport_height(viewport_height);

        if total_lines > viewport_height {
            let max_scroll = total_lines - viewport_height;
            state.scroll_offset = state.scroll_offset.min(max_scroll);
        } else {
            state.scroll_offset = 0;
        }

        let visible_text = state.text_buffer.display_window_lines(
            state.scroll_offset,
            state.scroll_offset + viewport_height,
        );

        let paragraph = Paragraph::new(visible_text).style(Style::default());
        Widget::render(paragraph, area, buf);

        if total_lines > viewport_height {
            self.render_scrollbar(buf, area, state);
        }
    }
}
