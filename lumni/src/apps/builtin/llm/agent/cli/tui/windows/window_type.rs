use std::fmt;

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    ResponseWindow,
    PromptWindow,
    CommandLine,
}

pub struct WindowType {
    kind: WindowKind,
    style: WindowStyle,
}

impl WindowType {
    pub fn new(kind: WindowKind, style: WindowStyle) -> Self {
        WindowType { kind, style }
    }
    
    // Function to provide a specific description for each window type
    pub fn description(&self) -> &str {
        match self.kind {
            WindowKind::ResponseWindow => "Response: View results and feedback here",
            WindowKind::PromptWindow => "Prompt: Enter your data",
            WindowKind::CommandLine => "",
        }
    }

    pub fn is_editable(&self) -> bool {
        match self.kind {
            WindowKind::ResponseWindow => false,
            WindowKind::PromptWindow => true,
            WindowKind::CommandLine => true,
        }
    }

    pub fn style(&self) -> WindowStyle {
        self.style
    }

    pub fn set_style(&mut self, style: WindowStyle) {
        self.style = style;
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStyle {
    Normal,
    Insert,
    Visual,
    InActive,
}

impl WindowStyle {
    pub fn block<'a>(&self, window_type: WindowType) -> Block<'a> {
        Block::default()
            .title(self.border_title(window_type))
            .borders(Borders::ALL)
            .border_style(self.border_style())
    }

    pub fn border_title(&self, window_type: WindowType) -> String {
        let base_title = match self {
            WindowStyle::Normal => format!("{} - type :q to quit, type i to enter insert mode", window_type.description()),
            WindowStyle::Insert => format!("Insert Mode - type Esc to back to normal mode"),
            WindowStyle::Visual => format!("Visual Mode - type y to yank, type c to cut, type Esc to back to normal mode"),
            WindowStyle::InActive => format!("{} - type :q to quit, type i to enter insert mode", window_type.description()),
        };

        base_title
    }

    pub fn border_style(&self) -> Style {
        match self {
            Self::Normal => Style::default().fg(Color::LightGreen),
            Self::Insert => Style::default().fg(Color::LightBlue),
            Self::Visual => Style::default().fg(Color::LightYellow),
            Self::InActive => Style::default().fg(Color::DarkGray),
        }
    }

    pub fn cursor_style(&self) -> Style {
        let color = match self {
            Self::Normal => Color::LightGreen,
            Self::Insert => Color::LightBlue,
            Self::Visual => Color::LightYellow,
            _ => return Style::default(),
        };
        Style::default().fg(color).add_modifier(Modifier::REVERSED)
    }
}

impl fmt::Display for WindowStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::InActive => write!(f, "NORMAL"), // InActive is just disabled Normal mode
        }
    }
}
