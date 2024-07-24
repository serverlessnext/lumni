use ratatui::style::{Color, Style};

use super::components::{
    TextSegment, TextWindow, TextWindowTrait, WindowConfig, WindowKind,
    WindowStatus, ReadWriteDocument,
};

pub struct PromptWindow<'a> {
    base: TextWindow<'a, ReadWriteDocument>,
}

impl<'a> TextWindowTrait<'a> for PromptWindow<'a> {
    fn base(&mut self) -> &mut TextWindow<'a, ReadWriteDocument> {
        &mut self.base
    }
}

impl PromptWindow<'_> {
    pub fn new() -> Self {
        let mut window_type = WindowConfig::new(WindowKind::PromptWindow);
        window_type.set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new_editable(window_type, None),
        }
    }
}

pub struct ResponseWindow<'a> {
    base: TextWindow<'a, ReadWriteDocument>,
}

impl<'a> TextWindowTrait<'a> for ResponseWindow<'a> {
    fn base(&mut self) -> &mut TextWindow<'a, ReadWriteDocument> {
        &mut self.base
    }
}

impl ResponseWindow<'_> {
    pub fn new(text: Option<Vec<TextSegment>>) -> Self {
        let mut window_type = WindowConfig::new(WindowKind::ResponseWindow);
        window_type.set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new_editable(window_type, text),
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

impl<'a> TextWindowTrait<'a> for CommandLine<'a> {
    fn base(&mut self) -> &mut TextWindow<'a, ReadWriteDocument> {
        &mut self.base
    }
}

impl CommandLine<'_> {
    pub fn new() -> Self {
        let mut window_type = WindowConfig::new(WindowKind::CommandLine);
        window_type.set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new_editable(window_type, None),
            mode: CommandLineMode::Normal,
        }
    }

    pub fn set_alert(&mut self, message: &str) {
        let style = Style::new().bg(Color::Red);
        self.text_set(&message, Some(style));
        self.mode = CommandLineMode::Alert;
    }

    pub fn set_normal_mode(&mut self) {
        if self.mode != CommandLineMode::Normal {
            self.text_empty();
            self.mode = CommandLineMode::Normal;
        }
    }
}
