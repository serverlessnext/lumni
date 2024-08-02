mod chat_session;
mod chat_session_manager;
mod conversation_loop;

use std::io;
use std::sync::Arc;

pub use chat_session::{ChatSession, ThreadedChatSession};
pub use chat_session_manager::ChatSessionManager;
pub use conversation_loop::prompt_app;
use lumni::api::error::ApplicationError;
use ratatui::backend::Backend;
use ratatui::Terminal;

use super::db::{ConversationDatabase, ConversationId};
use super::{
    db, draw_ui, AppUi, ColorScheme, ColorSchemeType, CommandLineAction,
    CompletionResponse, ConversationEvent, KeyEventHandler, ModalWindowType,
    ModelServer, PromptAction, PromptInstruction, ServerManager,
    TextWindowTrait, WindowEvent, WindowKind,
};
pub use crate::external as lumni;

pub struct App<'a> {
    pub ui: AppUi<'a>,
    pub chat_manager: ChatSessionManager,
    pub color_scheme: ColorScheme,
}

impl App<'_> {
    pub async fn new(
        initial_prompt_instruction: PromptInstruction,
        db_conn: Arc<ConversationDatabase>,
    ) -> Result<Self, ApplicationError> {
        let color_scheme = ColorScheme::new(ColorSchemeType::Default);

        let conversation_text = {
            let export =
                initial_prompt_instruction.export_conversation(&color_scheme);
            (!export.is_empty()).then(|| export)
        };

        let chat_manager = ChatSessionManager::new(
            initial_prompt_instruction,
            db_conn.clone(),
        )
        .await;

        log::debug!("Chat session manager created");

        let mut ui = AppUi::new(conversation_text);
        ui.init();

        Ok(App {
            ui,
            chat_manager,
            color_scheme,
        })
    }

    pub async fn reload_conversation(
        &mut self,
    ) -> Result<(), ApplicationError> {
        let prompt_instruction = self
            .chat_manager
            .get_active_session()
            .get_instruction()
            .await?;

        let conversation_text = {
            let export =
                prompt_instruction.export_conversation(&self.color_scheme);
            (!export.is_empty()).then(|| export)
        };

        if let Some(conversation_text) = conversation_text {
            self.ui.reload_conversation_text(conversation_text);
        };
        Ok(())
    }

    pub async fn draw_ui<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), io::Error> {
        // draw the UI in the terminal
        draw_ui(terminal, self).await?;

        // ensure the command line is (back) in normal mode afer drawing the UI
        // this ensures that an alert is automatically cleared on a subsequent key press
        self.ui.command_line.set_normal_mode();
        Ok(())
    }

    pub fn get_conversation_id_for_active_session(&self) -> &ConversationId {
        self.chat_manager.get_active_session_id()
    }

    pub async fn stop_active_chat_session(&mut self) {
        self.chat_manager.stop_active_chat_session();
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
}
