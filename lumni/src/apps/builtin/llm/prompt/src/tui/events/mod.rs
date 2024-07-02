mod handle_command_line;
mod handle_prompt_window;
mod handle_response_window;
mod key_event;
mod leader_key;
mod text_window_event;

pub use key_event::{KeyEventHandler, KeyTrack};

use super::clipboard::ClipboardProvider;
use super::components::{LineType, MoveCursor, TextWindowTrait, WindowKind};
use super::modal::ModalWindowType;
use super::ui::TabUi;
use super::TabSession;
use super::windows::PromptWindow;

#[derive(Debug)]
pub enum WindowEvent {
    Quit,
    PromptWindow,
    ResponseWindow,
    CommandLine(CommandLineAction),
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
    None,
    Write(String),
}
