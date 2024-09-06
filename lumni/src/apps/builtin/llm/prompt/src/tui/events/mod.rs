mod handle_command_line;
mod handle_prompt_window;
mod handle_response_window;
mod key_event;
mod leader_key;
mod text_window_event;

pub use key_event::{KeyEventHandler, KeyTrack};
use lumni::api::error::ApplicationError;

use super::clipboard::ClipboardProvider;
use super::modals::{ModalAction, ModalWindowType};
use super::ui::{AppUi, NavigationMode};
use super::window::{
    LineType, MoveCursor, PromptWindow, TextDocumentTrait, TextWindowTrait,
    WindowKind,
};
use super::{ConversationDbHandler, NewConversation, ThreadedChatSession};
pub use crate::external as lumni;

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    Quit,
    Conversation(ConversationWindowEvent),
    CommandLine(Option<CommandLineAction>),
    Prompt(PromptAction),
    Modal(ModalAction),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversationWindowEvent {
    Prompt(Option<ConversationEvent>),
    Response,
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
pub enum ConversationEvent {
    NewConversation(NewConversation),
    ReloadConversation, // only reload conversation
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserEvent {
    NewConversation(NewConversation), // prepare for future conversation
    ReloadConversation,               // reload conversation
    NewProfile,                       // prepare for future profile switch
    ReloadProfile,                    // prepare for future profile switch
}
