mod chat_session;
mod chat_session_manager;
mod conversation_loop;

use std::io;

use bytes::Bytes;
pub use chat_session::ChatSession;
pub use chat_session_manager::ChatSessionManager;
pub use conversation_loop::prompt_app;
use lumni::api::error::ApplicationError;
use ratatui::backend::Backend;
use ratatui::Terminal;

use super::db::{ConversationDbHandler, ConversationId};
use super::{
    db, draw_ui, AppUi, ColorScheme, ColorSchemeType, CommandLineAction,
    CompletionResponse, ConversationEvent, KeyEventHandler, ModalWindowType,
    ModelServer, PromptAction, PromptInstruction, ServerManager, TextLine,
    TextWindowTrait, WindowEvent, WindowKind,
};
pub use crate::external as lumni;

pub struct App<'a> {
    pub ui: AppUi<'a>,
    pub chat_manager: ChatSessionManager,
    pub color_scheme: ColorScheme,
}

impl App<'_> {
    pub fn new(
        initial_chat_session: ChatSession,
    ) -> Result<Self, ApplicationError> {
        let color_scheme = ColorScheme::new(ColorSchemeType::Default);
        let conversation_text = {
            let export =
                initial_chat_session.export_conversation(&color_scheme);
            (!export.is_empty()).then(|| export)
        };
        let mut ui = AppUi::new(conversation_text);
        ui.init();

        Ok(App {
            ui,
            chat_manager: ChatSessionManager::new(initial_chat_session),
            color_scheme,
        })
    }

    pub fn reload_conversation(&mut self) {
        let conversation_text = self
            .chat_manager
            .get_active_session()
            .export_conversation(&self.color_scheme);
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

    pub fn get_conversation_id_for_active_session(
        &mut self,
    ) -> Option<ConversationId> {
        self.chat_manager.get_active_session().get_conversation_id()
    }

    pub fn reset_active_session(
        &mut self,
        db_handler: &mut ConversationDbHandler<'_>,
    ) {
        self.chat_manager.get_active_session().reset(db_handler);
    }

    pub fn stop_active_chat_session(&mut self) {
        self.chat_manager.get_active_session().stop_chat_session();
    }

    pub async fn load_instruction_for_active_session(
        &mut self,
        prompt_instruction: PromptInstruction,
    ) -> Result<(), ApplicationError> {
        self.chat_manager
            .get_active_session()
            .load_instruction(prompt_instruction)
            .await
    }

    pub fn update_last_exchange_for_active_session(&mut self, content: &str) {
        self.chat_manager
            .get_active_session()
            .update_last_exchange(content);
    }

    pub fn process_response_for_active_session(
        &mut self,
        response: Bytes,
        start_of_stream: bool,
    ) -> Result<Option<CompletionResponse>, ApplicationError> {
        self.chat_manager
            .get_active_session()
            .process_response(response, start_of_stream)
    }
}
