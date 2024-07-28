mod modal;
mod config_modal;

use async_trait::async_trait;
use ratatui::Frame;
use ratatui::layout::Rect;

pub use super::SUPPORTED_MODEL_ENDPOINTS;

pub use super::{
    ApplicationError, ChatSession, ConversationEvent, ConversationReader,
    ModelServer, NewConversation, ServerManager, ServerTrait, WindowEvent,
    KeyTrack, Scroller,
};

pub use modal::ModalConfigWindow;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModalWindowType {
    Config,
}

#[async_trait]
pub trait ModalWindowTrait {
    fn get_type(&self) -> ModalWindowType;
    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect);
    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        tab_chat: &'a mut ChatSession,
        reader: Option<&ConversationReader<'_>>,
    ) -> Result<Option<WindowEvent>, ApplicationError>;
}