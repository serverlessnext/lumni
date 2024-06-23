
use super::components::{
    TextWindow, TextWindowTrait, WindowKind, WindowStatus,
    WindowType,
};

pub struct PromptWindow<'a> {
    base: TextWindow<'a>,
}

impl<'a> TextWindowTrait<'a> for PromptWindow<'a> {
    fn base(&mut self) -> &mut TextWindow<'a> {
        &mut self.base
    }
}

impl PromptWindow<'_> {
    pub fn new() -> Self {
        let window_type = WindowType::new(WindowKind::PromptWindow)
            .set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new(window_type),
        }
    }
}

pub struct ResponseWindow<'a> {
    base: TextWindow<'a>,
}

impl<'a> TextWindowTrait<'a> for ResponseWindow<'a> {
    fn base(&mut self) -> &mut TextWindow<'a> {
        &mut self.base
    }
}

impl ResponseWindow<'_> {
    pub fn new() -> Self {
        let window_type = WindowType::new(WindowKind::ResponseWindow)
            .set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new(window_type),
        }
    }
}

pub struct CommandLine<'a> {
    base: TextWindow<'a>,
}

impl<'a> TextWindowTrait<'a> for CommandLine<'a> {
    fn base(&mut self) -> &mut TextWindow<'a> {
        &mut self.base
    }
}

impl CommandLine<'_> {
    pub fn new() -> Self {
        let window_type = WindowType::new(WindowKind::CommandLine)
            .set_window_status(WindowStatus::InActive);
        Self {
            base: TextWindow::new(window_type),
        }
    }
}
