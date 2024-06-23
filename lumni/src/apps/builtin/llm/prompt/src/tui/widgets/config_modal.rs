use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::block::{Block, Padding};
use ratatui::widgets::{
    List, ListItem, Widget,
};

use super::SUPPORTED_MODEL_ENDPOINTS;

const MAX_WIDTH : u16 = 20;
const MAX_HEIGHT : u16 = 8;

pub struct ConfigModal {
    current_index: usize, // Current selection index
}

impl ConfigModal {
    pub fn new() -> Self {
        Self { current_index: 0 } // Initialize with the first item selected
    }

    pub fn max_area_size(&self) -> (u16, u16) {
        (MAX_WIDTH, MAX_HEIGHT)
    }


    pub fn key_down(&mut self) {
        // Increment the current index, wrapping around to 0 if past the last item
        self.current_index =
            (self.current_index + 1) % SUPPORTED_MODEL_ENDPOINTS.len();
    }

    pub fn key_up(&mut self) {
        self.current_index = if self.current_index == 0 {
            // get the last index if the current index is 0
            SUPPORTED_MODEL_ENDPOINTS.len().saturating_sub(1)
        } else {
            self.current_index.saturating_sub(1)
        };
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }
}

impl Widget for &mut ConfigModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Define the layout: a line of text and a list below it
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(4), // Space for the list
            ])
            .split(area);

        let background_block = Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title("Select Endpoint")
            .style(Style::default().bg(Color::Blue)); // Set the background color here

        background_block.render(area, buf); // Render the background block first

        // Prepare and render the list in the second chunk
        let items: Vec<ListItem> = SUPPORTED_MODEL_ENDPOINTS
            .iter()
            .enumerate()
            .map(|(index, &endpoint)| {
                let style = if index == self.current_index {
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Yellow) // Highlighted style
                } else {
                    Style::default().fg(Color::White) // Normal style
                };
                ListItem::new(Line::styled(endpoint, style))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .padding(Padding::uniform(1))
                    .style(Style::default().bg(Color::Black))
                    .borders(ratatui::widgets::Borders::NONE)
            );
        list.render(chunks[0], buf); 
    }
}
