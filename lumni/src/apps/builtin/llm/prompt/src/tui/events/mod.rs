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
use super::ui::{AppUi, ContentDisplayMode};
use super::window::{
    LineType, MoveCursor, PromptWindow, TextDocumentTrait, TextWindowTrait,
    WindowKind,
};
use super::{ConversationDbHandler, NewConversation, ThreadedChatSession};
pub use crate::external as lumni;

#[derive(Debug)]
pub enum WindowMode {
    Select,
    Conversation(Option<ConversationEvent>),
    FileBrowser(Option<FileBrowserEvent>),
    CommandLine(Option<CommandLineAction>),
    Prompt(PromptAction),
    Modal(ModalEvent),
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversationEvent {
    Select,
    Prompt,
    Response,
    NewConversation(NewConversation),
    ReloadConversation, // only reload conversation
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
