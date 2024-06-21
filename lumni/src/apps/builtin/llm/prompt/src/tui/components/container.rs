
use super::rect_area::RectArea;
use super::scroller::Scroller;
use super::{WindowType, WindowKind};

pub struct Container {
    area: RectArea,
    window_type: WindowType,
    scroller: Scroller,
}

impl Container {
    pub fn default() -> Self {
        Self {
            area: RectArea::default(),
            window_type: WindowType::new(WindowKind::Container),
            scroller: Scroller::new(),
       }
    }
}