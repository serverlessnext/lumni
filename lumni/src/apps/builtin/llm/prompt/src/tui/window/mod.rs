mod cursor;
mod scroller;
mod text_buffer;
mod text_display;
mod text_document;
mod text_render;
mod text_window;
mod window_config;

pub use cursor::MoveCursor;
use lumni::api::error::ApplicationError;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
pub use scroller::Scroller;
pub use text_buffer::TextBuffer;
pub use text_display::LineType;
pub use text_document::{
    ReadDocument, ReadWriteDocument, TextDocumentTrait, TextLine, TextSegment,
};
pub use text_window::{TextWindow, TextWindowTrait};
pub use window_config::{WindowConfig, WindowKind, WindowStatus};

pub use crate::external as lumni;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectArea {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl RectArea {
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

    pub fn update(
        &mut self,
        rect: &Rect,
        h_borders: bool,
        v_borders: bool,
    ) -> bool {
        // adjust widget area for borders
        // return true if updated, else false
        let previous = *self; // copy current state

        self.x = rect.x;
        self.y = rect.y;
        self.width = rect.width.saturating_sub(if h_borders { 2 } else { 0 });
        self.height = rect.height.saturating_sub(if v_borders { 2 } else { 0 });

        if *self != previous {
            true
        } else {
            false
        }
    }
}

pub struct PromptWindow<'a> {
    base: TextWindow<'a, ReadWriteDocument>,
}

impl<'a> TextWindowTrait<'a, ReadWriteDocument> for PromptWindow<'a> {
    fn base(&mut self) -> &mut TextWindow<'a, ReadWriteDocument> {
        &mut self.base
    }
}

impl PromptWindow<'_> {
    pub fn new() -> Self {
        let mut window_type = WindowConfig::new(WindowKind::PromptWindow);
        window_type.set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new_read_write(window_type, None),
        }
    }
}

pub struct ResponseWindow<'a> {
    base: TextWindow<'a, ReadDocument>,
}

impl<'a> TextWindowTrait<'a, ReadDocument> for ResponseWindow<'a> {
    fn base(&mut self) -> &mut TextWindow<'a, ReadDocument> {
        &mut self.base
    }
}

impl ResponseWindow<'_> {
    pub fn new(text: Option<Vec<TextLine>>) -> Self {
        let mut window_type = WindowConfig::new(WindowKind::ResponseWindow);
        window_type.set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new_read_append(window_type, text),
        }
    }
}

#[derive(Debug, PartialEq)]
enum CommandLineMode {
    Normal,
    Alert,
}

#[derive(Debug)]
pub struct CommandLine<'a> {
    base: TextWindow<'a, ReadWriteDocument>,
    mode: CommandLineMode,
}

impl<'a> TextWindowTrait<'a, ReadWriteDocument> for CommandLine<'a> {
    fn base(&mut self) -> &mut TextWindow<'a, ReadWriteDocument> {
        &mut self.base
    }
}

impl CommandLine<'_> {
    pub fn new() -> Self {
        let mut window_type = WindowConfig::new(WindowKind::CommandLine);
        window_type.set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new_read_write(window_type, None),
            mode: CommandLineMode::Normal,
        }
    }

    pub fn set_alert(&mut self, message: &str) -> Result<(), ApplicationError> {
        let style = Style::new().bg(Color::Red);
        self.text_set(&message, Some(style))?;
        self.mode = CommandLineMode::Alert;
        Ok(())
    }

    pub fn set_normal_mode(&mut self) {
        if self.mode != CommandLineMode::Normal {
            self.text_empty();
            self.mode = CommandLineMode::Normal;
        }
    }
}
