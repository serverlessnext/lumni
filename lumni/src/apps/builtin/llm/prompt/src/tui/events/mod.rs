mod key_event;
mod text_window_event;

pub use key_event::KeyEventHandler;

use super::clipboard::ClipboardProvider;
use super::components::{MoveCursor, TextWindowTrait, WindowKind};
use super::windows::{PromptWindow, ResponseWindow, CommandLine};

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
    Clear,
    Write(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandLineAction {
    None,
    Write(String),
}