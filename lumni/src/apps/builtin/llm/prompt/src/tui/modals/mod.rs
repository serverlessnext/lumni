mod conversations;
mod profiles;

use async_trait::async_trait;
pub use conversations::ConversationListModal;
pub use profiles::ProfileEditModal;
use ratatui::layout::Rect;
use ratatui::Frame;

use super::{
    ApplicationError, CommandLine, Conversation, ConversationDbHandler,
    ConversationEvent, ConversationStatus, KeyTrack, MaskMode,
    PromptInstruction, TextWindowTrait, ThreadedChatSession,
    UserProfileDbHandler, WindowEvent,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ModalWindowType {
    ConversationList(Option<ConversationEvent>),
    ProfileEdit,
}

#[async_trait]
pub trait ModalWindowTrait {
    fn get_type(&self) -> ModalWindowType;
    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect);
    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        tab_chat: &'a mut ThreadedChatSession,
        handler: &mut ConversationDbHandler,
    ) -> Result<Option<WindowEvent>, ApplicationError>;
}
