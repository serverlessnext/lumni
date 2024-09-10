//mod chat_session;
mod chat_session_manager;
mod conversation_loop;
mod threaded_chat_session;

use std::io;
use std::sync::Arc;

pub use chat_session_manager::{ChatEvent, ChatSessionManager};
pub use conversation_loop::prompt_app;
use lumni::api::error::ApplicationError;
use ratatui::backend::Backend;
use ratatui::Terminal;
pub use threaded_chat_session::ThreadedChatSession;

use super::db::{ConversationDatabase, ConversationId};
use super::{
    db, draw_ui, AppUi, ColorScheme, ColorSchemeType, CommandLineAction,
    CompletionResponse, ContentDisplayMode, ConversationEvent, Conversations,
    KeyEventHandler, ModalEvent, ModelServer, PromptAction, PromptError,
    PromptInstruction, PromptNotReadyReason, ServerManager, TextWindowTrait,
    UserEvent, WindowKind, WindowMode, Workspaces,
};
pub use crate::external as lumni;

pub struct App<'a> {
    pub ui: AppUi<'a>,
    pub chat_manager: ChatSessionManager,
    pub color_scheme: ColorScheme,
    pub is_processing: bool, // flag to indicate if the app is busy processing
}

impl App<'_> {
    pub async fn new(
        initial_prompt_instruction: Option<PromptInstruction>,
        db_conn: Arc<ConversationDatabase>,
    ) -> Result<Self, ApplicationError> {
        let color_scheme = ColorScheme::new(ColorSchemeType::Default);

        let conversation_text = initial_prompt_instruction
            .as_ref()
            .map(|instruction| {
                let export = instruction.export_conversation(&color_scheme);
                (!export.is_empty()).then(|| export)
            })
            .flatten();

        let chat_manager = ChatSessionManager::new(
            initial_prompt_instruction,
            db_conn.clone(),
        )
        .await;

        log::debug!("Chat session manager created");
        let handler = db_conn.get_conversation_handler(None);
        let conversations =
            Conversations::new(handler.fetch_conversation_list(100).await?);

        let workspaces = Workspaces::new_as_default(conversations);
        let mut ui = AppUi::new(workspaces, conversation_text).await;
        ui.init();

        Ok(App {
            ui,
            chat_manager,
            color_scheme,
            is_processing: false,
        })
    }

    pub async fn reload_conversation(
        &mut self,
    ) -> Result<(), ApplicationError> {
        let active_session =
            self.chat_manager.get_active_session()?.ok_or_else(|| {
                ApplicationError::NotReady(
                    "Reload failed. No active session available".to_string(),
                )
            })?;
        let prompt_instruction = active_session.get_instruction().await?;
        let conversation_text = {
            let export =
                prompt_instruction.export_conversation(&self.color_scheme);
            (!export.is_empty()).then(|| export)
        };

        if let Some(conversation_text) = conversation_text {
            // Update the conversation UI
            self.ui
                .conversation_ui
                .reload_conversation_text(conversation_text);
        } else {
            // If there's no conversation text, ensure we're in Conversation mode with an empty conversation
            self.ui.switch_to_conversation_mode();
        }

        Ok(())
    }

    pub async fn draw_ui<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        window_mode: &WindowMode,
    ) -> Result<(), io::Error> {
        // draw the UI in the terminal
        draw_ui(terminal, window_mode, self).await?;

        // ensure the command line is (back) in normal mode afer drawing the UI
        // this ensures that an alert is automatically cleared on a subsequent key press
        self.ui.command_line.set_normal_mode();
        Ok(())
    }

    pub fn get_conversation_id_for_active_session(
        &self,
    ) -> Option<ConversationId> {
        self.chat_manager.get_conversation_id_for_active_session()
    }

    pub async fn stop_active_chat_session(
        &mut self,
    ) -> Result<(), ApplicationError> {
        self.chat_manager.stop_active_chat_session()
    }

    pub async fn load_instruction_for_active_session(
        &mut self,
        prompt_instruction: PromptInstruction,
    ) -> Result<(), ApplicationError> {
        let active_session =
            self.chat_manager.get_active_session()?.ok_or_else(|| {
                ApplicationError::NotReady(
                    "Cant load instruction. No active session available"
                        .to_string(),
                )
            })?;

        active_session.load_instruction(prompt_instruction).await
    }
}
