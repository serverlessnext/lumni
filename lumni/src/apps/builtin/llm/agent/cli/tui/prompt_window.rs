use super::components::{
    TextWindow, TextWindowTrait, WindowKind, WindowStyle, WindowType,
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
            .set_style(WindowStyle::InActive);
        Self {
            base: TextWindow::new(window_type),
        }
    }
}
