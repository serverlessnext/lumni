mod command_line;
mod key_event;
mod text_window_event;

pub use key_event::KeyEventHandler;

use super::clipboard::ClipboardProvider;
use super::command_line::{transition_command_line, CommandLine};
use super::components::{MoveCursor, TextWindowTrait, WindowKind};
use super::prompt_window::PromptWindow;
use super::response_window::ResponseWindow;
use super::ChatSession;

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    Quit,
    PromptWindow,
    ResponseWindow,
    CommandLine,
    Prompt(PromptAction),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptAction {
    Clear,
    Write(String),
}
