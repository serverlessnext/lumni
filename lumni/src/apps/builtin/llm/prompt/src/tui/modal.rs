use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use ratatui::Frame;

use super::components::Scroller;
use super::events::KeyTrack;
use super::widgets::SelectEndpoint;
use super::WindowEvent;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModalWindowType {
    Config,
}

pub trait ModalWindowTrait {
    fn get_type(&self) -> ModalWindowType;
    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect);
    fn handle_key_event(
        &mut self,
        key_event: &mut KeyTrack,
    ) -> Option<WindowEvent>;
}

pub struct ModalConfigWindow {
    widget: SelectEndpoint,
    _scroller: Option<Scroller>,
}

impl ModalConfigWindow {
    pub fn new() -> Self {
        Self {
            widget: SelectEndpoint::new(),
            _scroller: None,
        }
    }
}

impl ModalWindowTrait for ModalConfigWindow {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::Config
    }

    fn render_on_frame(&mut self, frame: &mut Frame, mut area: Rect) {
        let (max_width, max_height) = self.widget.max_area_size();
        if area.width > max_width {
            area.x = area.width.saturating_sub(max_width);
            area.width = max_width;
        };
        if area.height > max_height {
            area.height = max_height;
        };
        frame.render_widget(Clear, area);
        frame.render_widget(&mut self.widget, area);
    }

    fn handle_key_event(
        &mut self,
        key_event: &mut KeyTrack,
    ) -> Option<WindowEvent> {
        match key_event.current_key().code {
            KeyCode::Up => self.widget.key_up(),
            KeyCode::Down => self.widget.key_down(),
            KeyCode::Enter => {
                let endpoint = self.widget.current_endpoint();
                eprintln!("Selected endpoint: {}", endpoint);
            }
            _ => {} // Ignore other keys
        }
        Some(WindowEvent::Modal(ModalWindowType::Config))
    }
}
