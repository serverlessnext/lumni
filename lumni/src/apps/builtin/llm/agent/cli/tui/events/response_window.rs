use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;
use tui_textarea::TextArea;

use super::key_event::KeyTrack;
use super::{
    ClipboardProvider, MoveCursor, ResponseWindow, 
    TextWindowExt, WindowEvent, WindowTrait,
};

pub fn handle_response_window_event(
    key_track: &KeyTrack,
    response_window: &mut ResponseWindow,
    command_line: &mut TextArea<'_>,
    _is_running: Arc<AtomicBool>,
) -> WindowEvent {
    let key_event = key_track.current_key();

    match key_event.code {
        KeyCode::Char(c) => {
            return handle_char_key(c, key_track, response_window, command_line);
        }
        KeyCode::Esc => {
            response_window.set_normal_mode();
        }
        KeyCode::Right => {
            response_window.move_cursor(MoveCursor::Right);
        }
        KeyCode::Left => {
            response_window.move_cursor(MoveCursor::Left);
        }
        KeyCode::Up => {
            response_window.move_cursor(MoveCursor::Up);
        }
        KeyCode::Down => {
            response_window.move_cursor(MoveCursor::Down);
        }

        // Default to stay in the same mode if no relevant key is pressed
        _ => {}
    }
    WindowEvent::ResponseWindow
}

fn handle_char_key(
    character: char,
    key_track: &KeyTrack,
    response_window: &mut ResponseWindow,
    command_line: &mut TextArea<'_>,
) -> WindowEvent {
    match character {
        '0' => { response_window.move_cursor(MoveCursor::BeginLine); }
        '$' => { response_window.move_cursor(MoveCursor::EndLine); }
        'y' => {
            // Check if the last command was also 'y'
            if let Some(prev) = key_track.previous_char() {
                if prev == "y" {
                    // TODO: Implement yank_line
                } else {
                    yank_highlighted_text(response_window);
                }
            } else {
                yank_highlighted_text(response_window);
            }
        }
        'g' => {
            // Check if the last command was also 'g'
            if let Some(prev) = key_track.previous_char() {
                if prev == "g" {
                    response_window.move_cursor(MoveCursor::TopOfFile);
                }
            }
        }
        'G' => { response_window.move_cursor(MoveCursor::EndOfFile); }
        'v' => { response_window.toggle_highlighting(); }  // enable visual mode
        ':' => {
            // Switch to command line mode on ":" key press
            command_line.insert_str(":");
            return WindowEvent::CommandLine;
        }
        _ => {} 
    }
    WindowEvent::ResponseWindow
}

fn yank_highlighted_text(response_window: &mut ResponseWindow) {
    let text = response_window.text_buffer().highlighted_text();
    let mut clipboard = ClipboardProvider::new();

    if let Err(e) = clipboard.write_line(text, false) {
        eprintln!("Clipboard error: {}", e);
    }
}
