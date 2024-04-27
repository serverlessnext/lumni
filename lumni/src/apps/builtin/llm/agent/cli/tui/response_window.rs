use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, ScrollbarState};

use super::components::{
    TextWindow, TextWindowTrait, WindowKind, WindowStyle, WindowType,
};

pub struct ResponseWindow<'a> {
    base: TextWindow<'a>,
}

impl<'a> TextWindowTrait<'a> for ResponseWindow<'a> {
    fn get_base(&mut self) -> &mut TextWindow<'a> {
        &mut self.base
    }
    fn vertical_scroll_bar_state(&mut self) -> &mut ScrollbarState {
        self.base.vertical_scroll_bar_state()
    }
    fn widget(&mut self, area: &Rect) -> Paragraph {
        self.base.widget(area)
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