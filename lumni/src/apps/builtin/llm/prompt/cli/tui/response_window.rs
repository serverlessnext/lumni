use super::components::{
    TextWindow, TextWindowTrait, WindowKind, WindowStyle, WindowType,
};

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
            .set_style(WindowStyle::InActive);
        Self {
            base: TextWindow::new(window_type),
        }
    }
}
