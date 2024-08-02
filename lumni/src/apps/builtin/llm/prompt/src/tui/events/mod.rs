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
    ChatSession, ConversationDbHandler, NewConversation, PromptInstruction,
    ThreadedChatSession,
};
pub use crate::external as lumni;

#[derive(Debug)]
pub enum WindowEvent {
    Quit,
    PromptWindow(Option<ConversationEvent>),
    ResponseWindow,
    CommandLine(Option<CommandLineAction>),
    Prompt(PromptAction),
    Modal(ModalWindowType),
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
