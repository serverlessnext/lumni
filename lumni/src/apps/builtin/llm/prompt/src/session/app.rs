use std::io;

use lumni::api::error::ApplicationError;
use ratatui::backend::Backend;
use ratatui::Terminal;

pub use crate::external as lumni;

use super::{
    AppUi, ChatSession, ColorScheme, ColorSchemeType,
    draw_ui,
};

pub struct App<'a> {
    pub ui: AppUi<'a>,
    pub chat: ChatSession,
    pub color_scheme: ColorScheme,
}

impl App<'_> {
    pub fn new(chat_session: ChatSession) -> Result<Self, ApplicationError> {
        let color_scheme = ColorScheme::new(ColorSchemeType::Default);
        let conversation_text = {
            let export = chat_session.export_conversation(&color_scheme);
            (!export.is_empty()).then(|| export)
        };
        let mut ui = AppUi::new(conversation_text);
        ui.init();

        Ok(App {
            ui,
            chat: chat_session,
            color_scheme,
        })
    }

    pub fn reload_conversation(&mut self) {
        let conversation_text = self.chat.export_conversation(&self.color_scheme);
        self.ui.reload_conversation_text(conversation_text);
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
