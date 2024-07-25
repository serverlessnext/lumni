use lumni::api::error::ApplicationError;
use ratatui::style::{Color, Style};

use super::components::{
    ReadDocument, ReadWriteDocument, TextSegment, TextWindow, TextWindowTrait,
    WindowConfig, WindowKind, WindowStatus,
};
pub use crate::external as lumni;

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

// TODO: change ReadWriteDocument to ReadDocument
// works partially 
// can change Document type to ReadDocument, but when an insert is called it will
// throw an ApplicationError::NotImplemented -- which is correct behavior as we
// should aim to remove any insert calls that are actually append
pub struct ResponseWindow<'a> {
    base: TextWindow<'a, ReadWriteDocument>,
}

impl<'a> TextWindowTrait<'a, ReadWriteDocument> for ResponseWindow<'a> {
    fn base(&mut self) -> &mut TextWindow<'a, ReadWriteDocument> {
        &mut self.base
    }
}

impl ResponseWindow<'_> {
    pub fn new(text: Option<Vec<TextSegment>>) -> Self {
        let mut window_type = WindowConfig::new(WindowKind::ResponseWindow);
        window_type.set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new_read_write(window_type, text),
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
