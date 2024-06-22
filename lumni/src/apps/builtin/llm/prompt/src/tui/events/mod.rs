mod handle_command_line;
mod handle_prompt_window;
mod handle_response_window;
mod key_event;
mod text_window_event;

pub use key_event::KeyEventHandler;

use super::clipboard::ClipboardProvider;
use super::components::{
    LineType, MoveCursor, TextWindowTrait, WindowKind,
};
use super::ui::AppUi;
use super::windows::{PromptWindow, ModalWindow};

#[derive(Debug)]
pub enum WindowEvent {
    Quit,
    PromptWindow,
    ResponseWindow,
    CommandLine(CommandLineAction),
    Prompt(PromptAction),
    Modal(ModalWindow),
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
