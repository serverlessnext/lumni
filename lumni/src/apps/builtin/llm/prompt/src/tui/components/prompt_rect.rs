use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl PromptRect {
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

    pub fn update(&mut self, area: &Rect) -> bool {
        // adjust widget area for borders
        // return true if updated, else false
        let previous = *self; // copy current state

        self.x = area.x;
        self.y = area.y;
        self.width = area.width.saturating_sub(2);
        self.height = area.height.saturating_sub(2);

        if *self != previous {
            true
        } else {
            false
        }
    }
}
