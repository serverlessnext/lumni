mod command_line;
mod key_event;
mod prompt_window;
mod response_window;

pub use key_event::KeyEventHandler;

pub use super::clipboard::ClipboardProvider;
pub use super::command_line::{transition_command_line, CommandLine};
pub use super::response_window::{
    ResponseWindow, TextWindowExt, TextWindowTrait,
};
pub use super::{ChatSession, MoveCursor, TextAreaHandler};

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
