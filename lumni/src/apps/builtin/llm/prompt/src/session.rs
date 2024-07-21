use std::io;

use lumni::api::error::ApplicationError;
use ratatui::backend::Backend;
use ratatui::Terminal;

use super::chat::ChatSession;
use super::tui::{draw_ui, ColorScheme, ColorSchemeType, TabUi};
pub use crate::external as lumni;

pub struct AppSession<'a> {
    tabs: Vec<TabSession<'a>>,
    _defaults: AppDefaults,
}

impl<'a> AppSession<'a> {
    pub fn new() -> Result<Self, ApplicationError> {
        Ok(AppSession {
            tabs: Vec::new(),
            _defaults: AppDefaults::new(),
        })
    }

    pub fn add_tab(&mut self, chat_session: ChatSession) {
        self.tabs.push(TabSession::new(chat_session));
    }

    pub fn get_tab_mut(&mut self, index: usize) -> Option<&mut TabSession<'a>> {
        self.tabs.get_mut(index)
    }

    pub fn _get_defaults(&self) -> &AppDefaults {
        &self._defaults
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
pub struct TabSession<'a> {
    pub ui: TabUi<'a>,
    pub chat: ChatSession,
    pub color_scheme: ColorScheme,
}

impl TabSession<'_> {
    fn new(chat: ChatSession) -> Self {
        let mut tab_ui = TabUi::new();
        tab_ui.init();
        TabSession {
            ui: tab_ui,
            chat,
            color_scheme: ColorScheme::new(ColorSchemeType::Default),
        }
    }

    pub fn new_conversation(&mut self, chat: ChatSession) {
        self.chat = chat;
    }

    pub fn draw_ui<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), io::Error> {
        // draw the UI in the terminal
        draw_ui(terminal, self)?;

        // ensure the command line is (back) in normal mode afer drawing the UI
        // this ensures that an alert is automatically cleared on a subsequent key press
        self.ui.command_line.set_normal_mode();
        Ok(())
    }
}
