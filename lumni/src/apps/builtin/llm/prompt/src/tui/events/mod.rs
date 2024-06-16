mod key_event;
mod text_window_event;

pub use key_event::KeyEventHandler;

use super::clipboard::ClipboardProvider;
use super::components::{MoveCursor, TextWindowTrait, WindowKind, LineType};
use super::windows::{CommandLine, PromptWindow, ResponseWindow};

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    Quit,
    PromptWindow,
    ResponseWindow,
    CommandLine(CommandLineAction),
    Prompt(PromptAction),
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
