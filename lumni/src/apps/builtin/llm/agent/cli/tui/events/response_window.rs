use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;
use tui_textarea::TextArea;

use super::key_event::KeyTrack;
use super::{MoveCursor, PromptLogWindow, WindowEvent};

pub fn handle_response_window_event(
    key_track: &KeyTrack,
    response_window: &mut PromptLogWindow,
    command_line: &mut TextArea<'_>,
    _is_running: Arc<AtomicBool>,
) -> WindowEvent {
    let key_event = key_track.current_key();

    match key_event.code {
        KeyCode::Char(':') => {
            // Switch to command line mode on ":" key press
            command_line.insert_str(":");
            WindowEvent::CommandLine
        }
        KeyCode::Right => {
            response_window.move_cursor(MoveCursor::Right);
            WindowEvent::ResponseWindow
        }
        KeyCode::Left => {
            response_window.move_cursor(MoveCursor::Left);
            WindowEvent::ResponseWindow
        }
        KeyCode::Up => {
            response_window.move_cursor(MoveCursor::Up);
            WindowEvent::ResponseWindow
        }
        KeyCode::Down => {
            response_window.move_cursor(MoveCursor::Down);
            WindowEvent::ResponseWindow
        }
        // Default to stay in the same mode if no relevant key is pressed
        _ => WindowEvent::ResponseWindow,
    }
}
