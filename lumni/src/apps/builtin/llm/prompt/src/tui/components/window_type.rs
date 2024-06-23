use ratatui::style::{Color, Style};
use ratatui::widgets::Borders;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStatus {
    Normal(Highlighted),
    Insert,
    Visual,
    InActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Highlighted {
    True,
    False,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    ResponseWindow,
    PromptWindow,
    CommandLine,
    Container,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowType {
    kind: WindowKind,
    status: WindowStatus,
}

impl WindowType {
    pub fn new(kind: WindowKind) -> Self {
        WindowType {
            kind,
            status: WindowStatus::InActive,
        }
    }

    // Function to provide a specific description for each window type
    pub fn description(&self) -> &str {
        match self.kind {
            WindowKind::ResponseWindow => "Chat",
            WindowKind::PromptWindow => "Prompt",
            WindowKind::CommandLine => "",
            WindowKind::Container => "",
        }
    }

    pub fn placeholder_text(&self) -> &str {
        match self.kind {
            WindowKind::ResponseWindow => "",
            WindowKind::PromptWindow => match self.status {
                WindowStatus::Normal(_) => "Press i to insert text",
                WindowStatus::Insert => "Type text and press Enter",
                WindowStatus::Visual => "",
                WindowStatus::InActive => "",
            },
            WindowKind::CommandLine => "Ready",
            WindowKind::Container => "",
        }
    }

    pub fn borders(&self) -> Borders {
        match self.kind {
            WindowKind::ResponseWindow => Borders::ALL,
            WindowKind::PromptWindow => Borders::ALL,
            WindowKind::CommandLine => Borders::NONE,
            WindowKind::Container => Borders::NONE,
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
            WindowStatus::Normal(highlighted) => match highlighted {
                Highlighted::True => Style::default().fg(Color::LightGreen),
                Highlighted::False => Style::default().fg(light_gray),
            },
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
            WindowKind::Container => false,
        }
    }

    pub fn kind(&self) -> WindowKind {
        self.kind
    }

    pub fn window_status(&self) -> WindowStatus {
        self.status
    }

    pub fn set_window_status(&mut self, status: WindowStatus) -> Self {
        self.status = status;
        *self
    }
}
