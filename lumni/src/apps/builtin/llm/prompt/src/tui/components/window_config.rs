use ratatui::layout::Alignment;
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets::block::{Title, Position};
use ratatui::widgets::Borders;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStatus {
    Normal,
    Background,
    Insert,
    Visual,
    InActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    ResponseWindow,
    PromptWindow,
    CommandLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    kind: WindowKind,
    status: WindowStatus,
    title: Option<String>,
}

impl WindowConfig {
    pub fn new(kind: WindowKind) -> Self {
        WindowConfig {
            kind,
            status: WindowStatus::InActive,
            title: None,
        }
    }

    pub fn title(&self) -> Option<Title> {
        if let Some(title) = &self.title {
            Some(Title::from(title.as_str()).alignment(Alignment::Left))
        } else {
            None
        }
    }

    pub fn hint(&self) -> Option<Title> {
        match self.kind {
            WindowKind::PromptWindow => {
                match self.status {
                    WindowStatus::Normal | WindowStatus::Background => {
                        Some(Title::from("press Enter to send prompt".dark_gray()).alignment(Alignment::Right).position(Position::Bottom))
                    }
                    WindowStatus::Insert => {
                        Some(Title::from("press Tab or Esc to exit insert mode".dark_gray()).alignment(Alignment::Right).position(Position::Bottom))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn set_title_text(&mut self, title: &str) -> &Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn placeholder_text(&self) -> &str {
        match self.kind {
            WindowKind::ResponseWindow => "",
            WindowKind::PromptWindow => match self.status {
                WindowStatus::Normal | WindowStatus::Background => "Press i to enter insert mode",
                WindowStatus::Visual => "",
                WindowStatus::InActive => "",
                WindowStatus::Insert => "Type text",
            },
            WindowKind::CommandLine => "Ready",
        }
    }

    pub fn borders(&self) -> Borders {
        match self.kind {
            WindowKind::ResponseWindow => Borders::ALL,
            WindowKind::PromptWindow => Borders::ALL,
            WindowKind::CommandLine => Borders::NONE,
        }
    }

    pub fn style(&self) -> Style {
        match self.kind {
            WindowKind::CommandLine => Style::default(),
            _ => Style::default().bg(Color::Black).fg(Color::White),
        }
    }

    pub fn border_style(&self) -> Style {
        let light_gray = Color::Rgb(128, 128, 128);
        let light_yellow = Color::Rgb(192, 192, 96);
        match self.status {
            WindowStatus::Normal => Style::default().fg(Color::LightGreen),
            WindowStatus::Background => Style::default().fg(light_gray),
            WindowStatus::Insert => Style::default().fg(Color::LightBlue),
            WindowStatus::Visual => Style::default().fg(light_yellow),
            WindowStatus::InActive => Style::default().fg(light_gray),
        }
    }

    pub fn is_editable(&self) -> bool {
        match self.kind {
            WindowKind::ResponseWindow => false,
            WindowKind::PromptWindow => true,
            WindowKind::CommandLine => true,
        }
    }

    pub fn kind(&self) -> WindowKind {
        self.kind
    }

    pub fn window_status(&self) -> WindowStatus {
        self.status
    }

    pub fn set_window_status(&mut self, status: WindowStatus) -> &Self {
        self.status = status;
        self
    }
}
