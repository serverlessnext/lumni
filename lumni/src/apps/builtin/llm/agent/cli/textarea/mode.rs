use std::fmt;

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    Visual,
    ReadOnly,
}

impl EditorMode {
    pub fn block<'a>(&self) -> Block<'a> {
        let help = match self {
            Self::Normal => "type :q to quit, type i to enter insert mode",
            Self::Insert => "type Esc to back to normal mode",
            Self::Visual => {
                "type y to yank, type c to cut, type Esc to back to normal mode"
            }
            Self::ReadOnly => "",
        };

        let title = if help.is_empty() {
            format!("{}", self)
        } else {
            format!("{} ({})", self, help)
        };
        Block::default().borders(Borders::ALL).title(title)
    }

    pub fn cursor_style(&self) -> Style {
        let color = match self {
            Self::Normal => Color::Reset,
            Self::Insert => Color::LightBlue,
            Self::Visual => Color::LightYellow,
            Self::ReadOnly => Color::DarkGray,
        };
        Style::default().fg(color).add_modifier(Modifier::REVERSED)
    }
}

impl fmt::Display for EditorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::ReadOnly => write!(f, ""),
        }
    }
}
