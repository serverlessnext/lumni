use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Widget};
use ratatui::Frame;

use super::components::Scroller;


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModalWindowType {
    Config,
}

pub struct ModalWindow {
    window_type: ModalWindowType,
    _scroller: Option<Scroller>,
}

impl ModalWindow {
    pub fn new(window_type: ModalWindowType) -> Self {
        Self { window_type, _scroller: None }
    }

    pub fn render_on_frame(&self, frame: &mut Frame, area: Rect) {
        let widget = match self.window_type {
            ModalWindowType::Config => ConfigWidget {
                text: "Initializing Config".to_string(),
            },
        };
        // render the widget on the frame
        frame.render_widget(Clear, area);
        frame.render_widget(widget, area);
    }
}

pub struct ConfigWidget {
    text: String,
}

impl Widget for ConfigWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = Span::styled(&self.text, Modifier::BOLD);
        let line = Line::from(vec![text]);
        line.render(area, buf);
    }
}
