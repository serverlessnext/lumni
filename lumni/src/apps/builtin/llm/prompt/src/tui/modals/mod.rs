mod conversations;
mod filebrowser;
mod settings;

use async_trait::async_trait;
pub use conversations::ConversationListModal;
pub use filebrowser::FileBrowserModal;
use ratatui::layout::Rect;
use ratatui::Frame;
pub use settings::SettingsModal;

pub use super::widgets;
use super::{
    ApplicationError, ChatSessionManager, Conversation, ConversationDbHandler,
    ConversationEvent, ConversationId, ConversationStatus, KeyTrack, MaskMode,
    ModalEvent, ModelServer, ModelSpec, PromptInstruction, ProviderConfig,
    ProviderConfigOptions, ReadDocument, ServerTrait, SimpleString, TextLine,
    TextSegment, ThreadedChatSession, UserEvent, UserProfile,
    UserProfileDbHandler, WindowMode, SUPPORTED_MODEL_ENDPOINTS,
};
use crate::apps::builtin::llm::prompt::src::chat;

#[derive(Debug, Clone, PartialEq)]
pub enum ModalWindowType {
    ProfileEdit,
    ConversationList,
    FileBrowser,
}

#[async_trait]
pub trait ModalWindowTrait: Send + Sync {
    fn get_type(&self) -> ModalWindowType;
    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect);
    async fn poll_background_task(
        &mut self,
    ) -> Result<WindowMode, ApplicationError> {
        // handle_key_event can return WindowEvent::Modal(ModalEvent::PollBackGroundTask),
        // this means a background process started, and must be monitored by calling this method. The monitoring can stop when a regular UpdateUI is received
        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
    }
    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        chat_manager: &mut ChatSessionManager,
        handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError>;
}
