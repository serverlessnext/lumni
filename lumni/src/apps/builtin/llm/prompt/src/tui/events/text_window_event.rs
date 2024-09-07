use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;
use lumni::api::error::ApplicationError;

use super::key_event::KeyTrack;
use super::leader_key::{process_leader_key, LEADER_KEY};
use super::{
    ClipboardProvider, CommandLineAction, ConversationEvent, MoveCursor,
    TextDocumentTrait, TextWindowTrait, WindowKind, WindowMode,
};
pub use crate::external as lumni;

pub fn handle_text_window_event<'a, T, D>(
    key_track: &mut KeyTrack,
    window: &mut T,
    _is_running: Arc<AtomicBool>,
) -> Result<WindowMode, ApplicationError>
where
    T: TextWindowTrait<'a, D>,
    D: TextDocumentTrait,
{
    let key_event = key_track.current_key();
    let is_insert_mode = window.is_status_insert();

    // in non-insert mode, enable leader_key if not yet enabled,
    // and the previous key was <leader>, and the current key is a char,
    // except current key is "i" (insert mode)
    if !is_insert_mode
        && !key_track.leader_key_set()
        && key_event.code == KeyCode::Char(LEADER_KEY)
    // leader key
    {
        key_track.set_leader_key(true); // enable leader key capture
    } else if key_track.leader_key_set() {
        // process captured leader key string
        if let Some(event) = process_leader_key(key_track) {
            return Ok(event);
        }
    } else {
        // process regular key
        match key_event.code {
            KeyCode::Char(c) => {
                // check mode
                if is_insert_mode {
                    window.text_insert_add(&c.to_string(), None)?;
                } else {
                    return handle_char_key(c, key_track, window);
                }
            }
            KeyCode::Esc => {
                window.set_status_normal();
            }
            KeyCode::Tab => {
                if is_insert_mode {
                    window.text_insert_add("        ", None)?;
                }
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
                if window.is_editable() {
                    if !is_insert_mode {
                        window.set_status_insert();
                    }
                    window.text_insert_add("\n", None)?;
                }
            }
            KeyCode::Backspace => {
                if window.is_editable() && !window.text_buffer().is_empty() {
                    window.text_delete_backspace()?;
                }
            }
            KeyCode::Delete => {
                if window.is_editable() {
                    window.text_delete_char()?;
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
    }

    let kind = match window.get_kind() {
        WindowKind::ResponseWindow => {
            WindowMode::Conversation(Some(ConversationEvent::Response))
        }
        WindowKind::EditorWindow => {
            WindowMode::Conversation(Some(ConversationEvent::Prompt))
        }
        WindowKind::CommandLine => WindowMode::CommandLine(None),
    };
    Ok(kind)
}

fn handle_char_key<'a, T, D>(
    character: char,
    key_track: &mut KeyTrack,
    window: &mut T,
) -> Result<WindowMode, ApplicationError>
where
    T: TextWindowTrait<'a, D>,
    D: TextDocumentTrait,
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
            if let Some(prev) = key_track.previous_key_str() {
                if prev == "g" {
                    window.move_cursor(MoveCursor::StartOfFile);
                }
            }
        }
        'G' => {
            window.move_cursor(MoveCursor::EndOfFile);
        }
        'j' => {
            let lines_to_move = key_track.take_numeric_input().unwrap_or(1);
            window.move_cursor(MoveCursor::Down(lines_to_move));
        }
        'k' => {
            let lines_to_move = key_track.take_numeric_input().unwrap_or(1);
            window.move_cursor(MoveCursor::Up(lines_to_move));
        }
        'v' => {
            // enable visual mode
            window.toggle_visual_mode();
        }
        'i' => {
            if window.is_editable() {
                window.set_status_insert();
            } else {
                // TODO: give feedback
            }
        }
        'p' => {
            if window.is_editable() {
                let mut clipboard = ClipboardProvider::new();
                if let Ok(text) = clipboard.read_text() {
                    window.text_insert_add(&text, None)?;
                }
            } else {
                // TODO: give feedback
                log::warn!("Cannot paste in non-editable mode");
            }
        }
        'u' => {
            if window.is_editable() {
                window.text_undo()?;
            }
        }
        'r' => {
            if window.is_editable() {
                window.text_redo()?;
            }
        }
        'y' => {
            // Check if the last command was also 'y'
            if let Some(prev) = key_track.previous_key_str() {
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
            return Ok(WindowMode::CommandLine(Some(
                CommandLineAction::Write(":".to_string()),
            )));
        }
        // ignore other characters
        _ => {}
    }
    let kind = match window.get_kind() {
        WindowKind::ResponseWindow => {
            WindowMode::Conversation(Some(ConversationEvent::Response))
        }
        WindowKind::EditorWindow => {
            WindowMode::Conversation(Some(ConversationEvent::Prompt))
        }
        WindowKind::CommandLine => WindowMode::CommandLine(None),
    };
    Ok(kind)
}

fn yank_text<'a, T, D>(window: &mut T, lines_to_yank: Option<usize>)
where
    T: TextWindowTrait<'a, D>,
    D: TextDocumentTrait,
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
        Err(e) => Err(e.to_string()),
    }
}
