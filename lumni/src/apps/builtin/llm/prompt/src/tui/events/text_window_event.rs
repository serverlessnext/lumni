use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;

use super::key_event::KeyTrack;
use super::{
    ClipboardProvider, CommandLineAction, MoveCursor, TextWindowTrait,
    WindowEvent, WindowKind,
};

pub fn handle_text_window_event<'a, T>(
    key_track: &mut KeyTrack,
    window: &mut T,
    _is_running: Arc<AtomicBool>,
) -> WindowEvent
where
    T: TextWindowTrait<'a>,
{
    let key_event = key_track.current_key();
    match key_event.code {
        KeyCode::Char(c) => {
            // check mode
            if window.is_status_insert() {
                window.text_insert_add(&c.to_string(), None);
            } else {
                return handle_char_key(c, key_track, window);
            }
        }
        KeyCode::Esc => {
            window.set_normal_mode();
        }
        KeyCode::Tab => {
            // same as Escape
            window.set_normal_mode();
        }
        KeyCode::Right => {
            window.move_cursor(MoveCursor::Right(1));
        }
        KeyCode::Left => {
            window.move_cursor(MoveCursor::Left(1));
        }
        KeyCode::Up => {
            window.move_cursor(MoveCursor::Up(1));
        }
        KeyCode::Down => {
            window.move_cursor(MoveCursor::Down(1));
        }
        KeyCode::Enter => {
            if window.window_type().is_editable() {
                window.text_insert_add("\n", None);
            }
        }
        KeyCode::Backspace => {
            if window.window_type().is_editable() {
                window.text_delete_backspace();
            }
        }
        KeyCode::Delete => {
            if window.window_type().is_editable() {
                window.text_delete_char();
            }
        }
        KeyCode::Home => {
            window.move_cursor(MoveCursor::StartOfLine);
        }
        KeyCode::End => {
            window.move_cursor(MoveCursor::EndOfLine);
        }
        // Default to stay in the s mode if no relevant key is pressed
        _ => {}
    }

    match window.window_type().kind() {
        WindowKind::ResponseWindow => WindowEvent::ResponseWindow,
        WindowKind::PromptWindow => WindowEvent::PromptWindow,
        WindowKind::CommandLine => {
            WindowEvent::CommandLine(CommandLineAction::None)
        }
    }
}

fn handle_char_key<'a, T>(
    character: char,
    key_track: &mut KeyTrack,
    window: &mut T,
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
            window.move_cursor(MoveCursor::Left(1));
        }
        'l' => {
            window.move_cursor(MoveCursor::Right(1));
        }
        'g' => {
            // Check if the last command was also 'g'
            if let Some(prev) = key_track.previous_char() {
                if prev == "g" {
                    window.move_cursor(MoveCursor::StartOfFile);
                }
            }
        }
        'G' => {
            window.move_cursor(MoveCursor::EndOfFile);
        }
        'j' => {
            let lines_to_move =
                key_track.take_numeric_input().unwrap_or(1) as u16;
            window.move_cursor(MoveCursor::Down(lines_to_move));
        }
        'k' => {
            let lines_to_move =
                key_track.take_numeric_input().unwrap_or(1) as u16;
            window.move_cursor(MoveCursor::Up(lines_to_move));
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
                    window.text_insert_add(&text, None);
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
                    // yy yanks the current line
                    let yanked_text =
                        window.text_buffer().yank_lines(1).join("\n");
                    if !yanked_text.is_empty() {
                        write_to_clipboard(&yanked_text).ok();
                    }
                } else {
                    yank_text(window, key_track.take_numeric_input());
                }
            } else {
                yank_text(window, key_track.take_numeric_input());
            }
        }
        ':' => {
            // Switch to command line mode on ":" key press
            return WindowEvent::CommandLine(CommandLineAction::Write(
                ":".to_string(),
            ));
        }
        // ignore other characters
        _ => {}
    }
    match window.window_type().kind() {
        WindowKind::ResponseWindow => WindowEvent::ResponseWindow,
        WindowKind::PromptWindow => WindowEvent::PromptWindow,
        WindowKind::CommandLine => {
            WindowEvent::CommandLine(CommandLineAction::None)
        }
    }
}

fn yank_text<'a, T>(window: &mut T, lines_to_yank: Option<usize>)
where
    T: TextWindowTrait<'a>,
{
    let text_buffer = window.text_buffer();
    let selected_text = text_buffer.yank_selected_text();

    if let Some(selected_text) = selected_text {
        if write_to_clipboard(&selected_text).is_ok() {
            window.text_unselect(); // Unselect text after successful yank
        }
    } else if let Some(lines) = lines_to_yank {
        let yanked_text = text_buffer.yank_lines(lines).join("\n");
        if !yanked_text.is_empty() {
            if write_to_clipboard(&yanked_text).is_ok() {
                return; // Successful yank, no need to unselect
            }
        }
    }
}

fn write_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = ClipboardProvider::new();

    match clipboard.write_line(text, false) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Clipboard error: {}", e);
            Err(e.to_string())
        }
    }
}
