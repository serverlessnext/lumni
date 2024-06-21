pub const DEFAULT_N_PREDICT: u32 = 1024; // max number of tokens to predict on prompt
pub const DEFAULT_TEMPERATURE: f64 = 0.8; // randomness of generated text

// only used when cant be fetched from the server, and not set by the user
pub const DEFAULT_CONTEXT_SIZE: usize = 512;

use ratatui::style::{Color, Style};

pub struct PromptStyle;

impl PromptStyle {
    pub fn user() -> Style {
        Style::new().fg(Color::White).bg(Color::Rgb(0, 64, 64))
    }

    pub fn assistant() -> Style {
        Style::new().fg(Color::Black).bg(Color::Rgb(225, 205, 175))
    }
}
