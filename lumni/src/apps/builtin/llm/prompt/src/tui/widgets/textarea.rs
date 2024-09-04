use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{
    Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
    StatefulWidgetRef, Widget,
};
use ratatui::Frame;

use super::{
    KeyTrack, ReadDocument, ReadWriteDocument, TextBuffer, TextDocumentTrait,
    TextLine,
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
}

impl<T: TextDocumentTrait> TextAreaWidget<T> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<'a, T: TextDocumentTrait> TextAreaState<'a, T> {
    pub fn new(document: T) -> Self {
        Self {
            text_buffer: TextBuffer::new(document),
            key_track: KeyTrack::new(),
            scroll_offset: 0,
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

        // For now, just print the received key events
        eprintln!(
            "Received key event for TextArea: {:?}",
            self.key_track.current_key()
        );
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

        let visible_text = state.text_buffer.display_window_lines(
            state.scroll_offset,
            state.scroll_offset + area.height as usize,
        );

        let paragraph = Paragraph::new(visible_text).style(Style::default());
        Widget::render(paragraph, area, buf);
    }
}
