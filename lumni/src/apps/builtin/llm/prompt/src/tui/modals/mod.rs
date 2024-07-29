mod conversations;
mod endpoint;

use async_trait::async_trait;
pub use conversations::ConversationListModal;
pub use endpoint::SelectEndpointModal;
use ratatui::layout::Rect;
use ratatui::Frame;

pub use super::{
    ApplicationError, ChatSession, Conversation, ConversationEvent,
    ConversationDbHandler, KeyTrack, ModelServer, NewConversation,
    PromptInstruction, Scroller, ServerManager, ServerTrait, WindowEvent,
    SUPPORTED_MODEL_ENDPOINTS,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModalWindowType {
    SelectEndpoint,
    ConversationList,
}

#[async_trait]
pub trait ModalWindowTrait {
    fn get_type(&self) -> ModalWindowType;
    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect);
    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        tab_chat: &'a mut ChatSession,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError>;
}
