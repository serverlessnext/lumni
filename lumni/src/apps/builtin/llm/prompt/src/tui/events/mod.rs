mod handle_command_line;
mod handle_prompt_window;
mod handle_response_window;
mod key_event;
mod leader_key;
mod text_window_event;

pub use key_event::{KeyEventHandler, KeyTrack};
use lumni::api::error::ApplicationError;

use super::clipboard::ClipboardProvider;
use super::modals::ModalWindowType;
use super::ui::AppUi;
use super::window::{
    LineType, MoveCursor, PromptWindow, TextDocumentTrait, TextWindowTrait,
    WindowKind,
};
use super::{
    ChatSessionManager, ConversationDbHandler, NewConversation,
    ThreadedChatSession,
};
pub use crate::external as lumni;

#[derive(Debug, PartialEq)]
pub enum WindowMode {
    Select,
    Conversation(Option<ConversationEvent>),
    FileBrowser(Option<FileBrowserEvent>),
    CommandLine(Option<CommandLineAction>),
    Prompt(PromptAction),
    Modal(ModalEvent),
    Alert(String),
    Quit,
}

impl Default for WindowMode {
    fn default() -> Self {
        WindowMode::Conversation(Some(ConversationEvent::PromptRead))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversationEvent {
    Select(Option<ConversationSelectEvent>),
    PromptInsert,
    PromptRead,
    Response,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversationSelectEvent {
    NewConversation(NewConversation),
    ReloadConversation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileBrowserEvent {
    Select,
    Search,
    Quit,
    OpenFile,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptAction {
    Stop,          // stop stream
    Write(String), // send prompt
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandLineAction {
    Write(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalEvent {
    Open(ModalWindowType), // open the modal
    PollBackGroundTask,    // modal needs to be polled for background updates
    UpdateUI, // update the UI of the modal once and wait for the next key event
    Close,    // close the current modal
    Event(UserEvent),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserEvent {
    NewConversation(NewConversation), // prepare for future conversation
    ReloadConversation,               // reload conversation
    NewProfile,                       // prepare for future profile switch
    ReloadProfile,                    // prepare for future profile switch
}
