use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, ScrollbarState};

use super::components::{
    TextWindow, TextWindowTrait, WindowKind, WindowStyle, WindowType,
};

pub struct ResponseWindow<'a> {
    base: TextWindow<'a>,
    is_active: bool,
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
        let window_type =
            WindowType::new(WindowKind::ResponseWindow, WindowStyle::InActive);
        Self {
            base: TextWindow::new(window_type),
            is_active: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn set_active(&mut self, active: bool) {
        // change style based on active state
        if active {
            self.base.set_window_style(WindowStyle::Normal);
        } else {
            self.base.set_window_style(WindowStyle::InActive);
        }
        self.is_active = active;
    }
}

//impl<'a> TextWindowBaseTrait<'a> for ResponseWindow<'a> {
//    fn get_base(&mut self) -> &mut TextWindow<'a> {
//        &mut self.base
//    }
//}
