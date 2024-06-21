use ratatui::layout::Rect;

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

    pub fn update(&mut self, rect: &Rect) -> bool {
        // adjust widget area for borders
        // return true if updated, else false
        let previous = *self; // copy current state

        self.x = rect.x;
        self.y = rect.y;
        self.width = rect.width.saturating_sub(2);
        self.height = rect.height.saturating_sub(2);

        if *self != previous {
            true
        } else {
            false
        }
    }
}
