mod key_event;
mod command_line;
mod prompt_window;
mod response_window;

pub use super::response_window::PromptLogWindow;
pub use super::{TextAreaHandler, MoveCursor};
pub use super::command_line::{transition_command_line, CommandLine};

pub use key_event::process_key_event;

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