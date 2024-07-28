mod handle_command_line;
mod handle_prompt_window;
mod handle_response_window;
mod key_event;
mod leader_key;
mod text_window_event;

pub use key_event::{KeyEventHandler, KeyTrack};
use lumni::api::error::ApplicationError;

use super::clipboard::ClipboardProvider;
use super::window::{
    LineType, MoveCursor, TextDocumentTrait, TextWindowTrait, WindowKind,
};
use super::modals::ModalWindowType;
use super::ui::TabUi;
use super::window::PromptWindow;
use super::{ChatSession, ConversationReader, NewConversation};
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
    Clear,         // stop stream and clear prompt
    Write(String), // send prompt
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandLineAction {
    Write(String),
}

#[derive(Debug)]
pub enum ConversationEvent {
    NewConversation(NewConversation),
}
