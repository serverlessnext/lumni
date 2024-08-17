mod conversations;
mod profiles;

use async_trait::async_trait;
pub use conversations::ConversationListModal;
pub use profiles::ProfileEditModal;
use ratatui::layout::Rect;
use ratatui::Frame;

use super::{
    ApplicationError, CommandLine, Conversation, ConversationDbHandler,
    ConversationStatus, KeyTrack, MaskMode, ModelServer, ModelSpec,
    PromptInstruction, ServerTrait, TextWindowTrait, ThreadedChatSession,
    UserEvent, UserProfileDbHandler, WindowEvent, SUPPORTED_MODEL_ENDPOINTS,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ModalWindowType {
    ConversationList,
    ProfileEdit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalAction {
    Open(ModalWindowType), // open the modal
    Refresh,               // modal needs to be refreshed to handle updates
    WaitForKeyEvent,       // wait for a key event
    Close,                 // close the curren modal
    Event(UserEvent),
}

#[async_trait]
pub trait ModalWindowTrait: Send + Sync {
    fn get_type(&self) -> ModalWindowType;
    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect);
    async fn refresh(&mut self) -> Result<WindowEvent, ApplicationError> {
        // handle_key_event can return WindowEvent::Modal(ModalAction::Refresh),
        // which typically means a background process started, and can be monitored by calling refresh. Refresh can also return a WindowEvent with Action::Refresh. When background processes is completed, or none are running, it should return the default WaitForKeyEvent.
        // Default implementation completes immediately
        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
    }
    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        tab_chat: &'a mut ThreadedChatSession,
        handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError>;
}
