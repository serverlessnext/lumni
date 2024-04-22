mod command_line;
mod key_event;
mod prompt_window;
mod response_window;

pub use key_event::process_key_event;

pub use super::command_line::{transition_command_line, CommandLine};
pub use super::response_window::PromptLogWindow;
pub use super::{MoveCursor, TextAreaHandler};

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
