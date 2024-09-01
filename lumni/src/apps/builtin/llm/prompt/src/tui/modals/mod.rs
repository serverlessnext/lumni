mod conversations;
mod filebrowser;
mod settings;

use async_trait::async_trait;
pub use conversations::ConversationListModal;
pub use filebrowser::FileBrowserModal;
use ratatui::layout::Rect;
use ratatui::Frame;
pub use settings::SettingsModal;
use widgets::FileBrowserWidget;

pub use super::widgets;
use super::{
    ApplicationError, Conversation, ConversationDbHandler, ConversationStatus,
    KeyTrack, MaskMode, ModelServer, ModelSpec, PromptInstruction,
    ProviderConfig, ProviderConfigOptions, ResponseWindow, ServerTrait,
    SimpleString, TextArea, TextLine, TextWindowTrait, ThreadedChatSession,
    UserEvent, UserProfile, UserProfileDbHandler, WindowEvent,
    SUPPORTED_MODEL_ENDPOINTS,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ModalWindowType {
    ConversationList,
    ProfileEdit,
    FileBrowser,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalAction {
    Open(ModalWindowType), // open the modal
    PollBackGroundTask,    // modal needs to be polled for background updates
    UpdateUI, // update the UI of the modal once and wait for the next key event
    Close,    // close the curren modal
    Event(UserEvent),
}

#[async_trait]
pub trait ModalWindowTrait: Send + Sync {
    fn get_type(&self) -> ModalWindowType;
    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect);
    async fn poll_background_task(
        &mut self,
    ) -> Result<WindowEvent, ApplicationError> {
        // handle_key_event can return WindowEvent::Modal(ModalAction::PollBackGroundTask),
        // this means a background process started, and must be monitored by calling this method. The monitoring can stop when a regular UpdateUI is received
        Ok(WindowEvent::Modal(ModalAction::UpdateUI))
    }
    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        tab_chat: Option<&'a mut ThreadedChatSession>,
        handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError>;
}
