
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyEvent;
use tokio::sync::mpsc;
use tui_textarea::TextArea;
use bytes::Bytes;

use super::{PromptLogWindow, TextAreaHandler, 
    WindowEvent,
    CommandLine,
};

use super::response_window::handle_response_window_event;
use super::command_line::handle_command_line_event;
use super::prompt_window::handle_prompt_window_event;

pub async fn process_key_event(
    key_event: KeyEvent,
    current_mode: WindowEvent,
    command_line_handler: &mut CommandLine,
    command_line: &mut TextArea<'_>,
    editor_window: &mut TextAreaHandler,
    is_running: Arc<AtomicBool>,
    tx: mpsc::Sender<Bytes>,
    response_window: &mut PromptLogWindow<'_>,
) -> WindowEvent {
    match current_mode {
        WindowEvent::CommandLine => {
            handle_command_line_event(
                key_event,
                command_line_handler,
                response_window,
                editor_window,
                command_line,
                tx,
                is_running,
            ).await
        }
        WindowEvent::ResponseWindow => {
            handle_response_window_event(
                key_event,
                response_window,
                command_line,
                is_running,
            )
        },
        WindowEvent::PromptWindow => {
            handle_prompt_window_event(
                key_event,
                response_window,
                editor_window,
                command_line,
                tx,
                is_running,
            ).await
        }
        _ => current_mode,
    }
}