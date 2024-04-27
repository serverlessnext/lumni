use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;
use tui_textarea::TextArea;

use super::key_event::KeyTrack;
use super::{
    ClipboardProvider, MoveCursor, TextWindowTrait, WindowEvent, WindowKind,
};

pub fn handle_text_window_event<'a, T>(
    key_track: &mut KeyTrack,
    window: &mut T,
    command_line: &mut TextArea<'_>,
    _is_running: Arc<AtomicBool>,
) -> WindowEvent
where
    T: TextWindowTrait<'a>,
{
    let key_event = key_track.current_key();

    match key_event.code {
        KeyCode::Char(c) => {
            // check mode
            if window.is_style_insert() {
                window.text_insert_add(&c.to_string());
            } else {
                return handle_char_key(c, key_track, window, command_line);
            }
        }
        KeyCode::Esc => {
            if window.is_style_insert() {
                // commit
                window.text_insert_commit();
            }
            window.set_normal_mode();
        }
        KeyCode::Right => {
            window.move_cursor(MoveCursor::Right);
        }
        KeyCode::Left => {
            window.move_cursor(MoveCursor::Left);
        }
        KeyCode::Up => {
            window.move_cursor(MoveCursor::Up);
        }
        KeyCode::Down => {
            window.move_cursor(MoveCursor::Down);
        }
        // Default to stay in the same mode if no relevant key is pressed
        _ => {}
    }

    match window.window_type().kind() {
        WindowKind::ResponseWindow => WindowEvent::ResponseWindow,
        WindowKind::PromptWindow => WindowEvent::PromptWindow,
        WindowKind::CommandLine => WindowEvent::CommandLine,
    }
}

fn handle_char_key<'a, T>(
    character: char,
    key_track: &mut KeyTrack,
    window: &mut T,
    command_line: &mut TextArea<'_>,
) -> WindowEvent
where
    T: TextWindowTrait<'a>,
{
    match character {
        '0' => {
            window.move_cursor(MoveCursor::StartOfLine);
        }
        '$' => {
            window.move_cursor(MoveCursor::EndOfLine);
        }
        'h' => {
            window.move_cursor(MoveCursor::Left);
        }
        'l' => {
            window.move_cursor(MoveCursor::Right);
        }
        'g' => {
            // Check if the last command was also 'g'
            if let Some(prev) = key_track.previous_char() {
                if prev == "g" {
                    window.move_cursor(MoveCursor::TopOfFile);
                }
            }
        }
        'G' => {
            window.move_cursor(MoveCursor::EndOfFile);
        }
        'j' => {
            let lines_to_move =
                key_track.retrieve_and_reset_numeric_input() as u16;
            window.move_cursor(MoveCursor::LinesForward(lines_to_move));
        }
        'k' => {
            let lines_to_move =
                key_track.retrieve_and_reset_numeric_input() as u16;
            window.move_cursor(MoveCursor::LinesBackward(lines_to_move));
        }
        'v' => {
            // enable visual mode
            window.toggle_visual_mode();
        }
        'i' => {
            if window.window_type().is_editable() {
                window.set_insert_mode();
            } else {
                // TODO: give feedback
            }
        }
        'p' => {
            if window.window_type().is_editable() {
                let mut clipboard = ClipboardProvider::new();
                if let Ok(text) = clipboard.read_text() {
                    window.text_insert_add(&text);
                    window.text_insert_commit();
                }
            } else {
                // TODO: give feedback
                // eprintln!("Cannot paste in a read-only window");
            }
        }
        'u' => {
            if window.window_type().is_editable() {
                window.text_undo();
            }
        }
        'r' => {
            if window.window_type().is_editable() {
                window.text_redo();
            }
        }
        'y' => {
            // Check if the last command was also 'y'
            if let Some(prev) = key_track.previous_char() {
                if prev == "y" {
                    // TODO: Implement yank_line
                } else {
                    yank_highlighted_text(window);
                }
            } else {
                yank_highlighted_text(window);
            }
        }
        ':' => {
            // Switch to command line mode on ":" key press
            command_line.insert_str(":");
            return WindowEvent::CommandLine;
        }
        // ignore other characters
        _ => {}
    }
    match window.window_type().kind() {
        WindowKind::ResponseWindow => WindowEvent::ResponseWindow,
        WindowKind::PromptWindow => WindowEvent::PromptWindow,
        WindowKind::CommandLine => WindowEvent::CommandLine,
    }
}

fn yank_highlighted_text<'a, T>(window: &mut T)
where
    T: TextWindowTrait<'a>,
{
    let text = window.text_buffer().selected_text();
    let mut clipboard = ClipboardProvider::new();

    if let Err(e) = clipboard.write_line(text, false) {
        eprintln!("Clipboard error: {}", e);
    }
}
