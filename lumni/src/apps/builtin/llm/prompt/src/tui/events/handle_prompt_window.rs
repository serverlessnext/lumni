use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};
use lumni::api::error::ApplicationError;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    AppUi, ConversationWindowEvent, LineType, NavigationMode, PromptAction,
    PromptWindow, TextWindowTrait, WindowEvent,
};
use crate::apps::builtin::llm::prompt::src::tui::WindowKind;
pub use crate::external as lumni;

pub fn handle_prompt_window_event(
    app_ui: &mut AppUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Result<WindowEvent, ApplicationError> {
    let conv_ui = match &mut app_ui.selected_mode {
        NavigationMode::Conversation(ui) => ui,
        _ => {
            return Err(ApplicationError::InvalidState(
                "Cant use prompt window. Not in Conversation mode".to_string(),
            ))
        }
    };

    match key_track.current_key().code {
        KeyCode::Up => {
            if !conv_ui.response.text_buffer().is_empty() {
                let (_, row) = conv_ui.prompt.get_column_row();
                if row == 0 {
                    // jump from prompt window to response window
                    return Ok(conv_ui.set_response_window());
                }
            }
        }
        KeyCode::Tab => {
            if !in_editing_block(&mut conv_ui.prompt) {
                return Ok(conv_ui.prompt.next_window_status());
            }
        }
        KeyCode::Enter => {
            // handle enter if not in editing mode
            if !conv_ui.prompt.is_status_insert() {
                let question = conv_ui.prompt.text_buffer().to_string();
                return Ok(WindowEvent::Prompt(PromptAction::Write(question)));
            }
        }
        KeyCode::Backspace => {
            if conv_ui.prompt.text_buffer().is_empty() {
                return Ok(conv_ui.set_prompt_window(false));
            }
            if !conv_ui.prompt.is_status_insert() {
                // change to insert mode
                conv_ui.prompt.set_status_insert();
            }
        }
        KeyCode::Esc => {
            // ensure blocks are closed if inside editing block
            if conv_ui.prompt.is_status_insert() {
                ensure_closed_block(&mut conv_ui.prompt)?;
            }
            return Ok(conv_ui.set_prompt_window(false));
        }
        KeyCode::Char(key) => {
            // catch Ctrl + shortcut key
            if key_track.current_key().modifiers == KeyModifiers::CONTROL {
                match key {
                    'c' => {
                        if conv_ui.prompt.text_buffer().is_empty() {
                            return Ok(WindowEvent::Quit);
                        } else {
                            conv_ui.prompt.text_empty();
                        }
                    }
                    'q' => {
                        return Ok(WindowEvent::Quit);
                    }
                    'a' => {
                        conv_ui.prompt.text_select_all();
                    }
                    'j' => {
                        conv_ui.prompt.text_insert_add("\n", None)?;
                    }
                    _ => {}
                }
                return Ok(WindowEvent::Conversation(
                    ConversationWindowEvent::Prompt(None),
                ));
            } else if !conv_ui.prompt.is_status_insert() {
                // process regular key
                match key {
                    't' | 'T' => {
                        return Ok(conv_ui.set_response_window());
                    }
                    'i' | 'I' => {
                        return Ok(conv_ui.set_prompt_window(true));
                    }
                    '+' => {
                        conv_ui.set_primary_window(WindowKind::EditorWindow);
                    }
                    '-' => {
                        conv_ui.set_primary_window(WindowKind::ResponseWindow);
                    }
                    ' ' => {
                        if let Some(prev) = key_track.previous_key_str() {
                            if prev == " " {
                                // change to insert mode if double space
                                return Ok(conv_ui.set_prompt_window(true));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    handle_text_window_event(key_track, &mut conv_ui.prompt, is_running)
}

fn is_closed_block(prompt_window: &mut PromptWindow) -> Option<bool> {
    // return None if not inside a block
    // return Some(true) if block is closed, else return Some(false)
    let code_block = prompt_window.current_code_block();
    match code_block {
        Some(block) => Some(block.is_closed()),
        None => None,
    }
}

fn ensure_closed_block(
    prompt_window: &mut PromptWindow,
) -> Result<(), ApplicationError> {
    if let Some(closed_block) = is_closed_block(prompt_window) {
        if !closed_block {
            // close block
            prompt_window.text_append_with_insert("```", None)?;
        }
    }
    Ok(())
}

fn in_editing_block(prompt_window: &mut PromptWindow) -> bool {
    let line_type = prompt_window.current_line_type().unwrap_or(LineType::Text);
    match line_type {
        LineType::Code(block_line) => !block_line.is_end(),
        _ => false,
    }
}
