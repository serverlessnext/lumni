use std::io;

use ratatui::backend::Backend;
use ratatui::Terminal;

use super::chat::ChatSession;
use super::tui::{draw_ui, TabUi};

pub struct TabSession<'a> {
    pub ui: TabUi<'a>,
    pub chat: ChatSession,
}

impl TabSession<'_> {
    pub fn new(chat: ChatSession) -> Self {
        let mut tab_ui = TabUi::new();
        tab_ui.init();

        TabSession { ui: tab_ui, chat }
    }

    pub fn draw_ui<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), io::Error> {
        draw_ui(terminal, self)
    }
}

pub struct AppSession<'a> {
    tabs: Vec<TabSession<'a>>,
}

impl<'a> AppSession<'a> {
    pub fn new() -> Self {
        AppSession { tabs: Vec::new() }
    }

    pub fn add_tab(&mut self, chat_session: ChatSession) {
        self.tabs.push(TabSession::new(chat_session));
    }

    pub fn get_tab_mut(&mut self, index: usize) -> Option<&mut TabSession<'a>> {
        self.tabs.get_mut(index)
    }
}
