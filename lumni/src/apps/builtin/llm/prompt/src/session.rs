use std::io;

use ratatui::backend::Backend;
use ratatui::Terminal;

use super::chat::ChatSession;
use super::tui::{
    draw_ui, ColorScheme, ColorSchemeType, TabUi, TextWindowTrait,
};

pub struct TabSession<'a> {
    pub ui: TabUi<'a>,
    pub chat: ChatSession,
    pub color_scheme: Option<ColorScheme>,
}

impl TabSession<'_> {
    pub fn new(chat: ChatSession) -> Self {
        let mut tab_ui = TabUi::new();
        tab_ui.init();
        TabSession {
            ui: tab_ui,
            chat,
            color_scheme: Some(ColorScheme::new(ColorSchemeType::Default)),
        }
    }

    pub fn draw_ui<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), io::Error> {
        // Set the response window title to current server name
        self.ui.response.set_window_title(self.chat.server_name());

        // draw the UI in the terminal
        draw_ui(terminal, self)?;

        // ensure the command line is (back) in normal mode afer drawing the UI
        // this ensures that an alert is automatically cleared on a subsequent key press
        self.ui.command_line.set_normal_mode();
        Ok(())
    }
}

pub struct AppSession<'a> {
    tabs: Vec<TabSession<'a>>,
    defaults: AppDefaults,
}

impl<'a> AppSession<'a> {
    pub fn new() -> Self {
        AppSession {
            tabs: Vec::new(),
            defaults: AppDefaults::new(),
        }
    }

    pub fn add_tab(&mut self, chat_session: ChatSession) {
        self.tabs.push(TabSession::new(chat_session));
    }

    pub fn get_tab_mut(&mut self, index: usize) -> Option<&mut TabSession<'a>> {
        self.tabs.get_mut(index)
    }

    pub fn get_defaults(&self) -> &AppDefaults {
        &self.defaults
    }
}

#[derive(Debug, Clone)]
pub struct AppDefaults {
    color_scheme: ColorScheme,
}

impl AppDefaults {
    fn new() -> Self {
        AppDefaults {
            color_scheme: ColorScheme::new(ColorSchemeType::Default),
        }
    }

    pub fn get_color_scheme(&self) -> ColorScheme {
        self.color_scheme
    }
}
