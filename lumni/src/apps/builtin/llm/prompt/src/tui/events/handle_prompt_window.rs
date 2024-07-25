use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};
use lumni::api::error::ApplicationError;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    LineType, PromptAction, PromptWindow, TabUi, TextWindowTrait, WindowEvent,
};
use crate::apps::builtin::llm::prompt::src::tui::WindowKind;
pub use crate::external as lumni;

pub fn handle_prompt_window_event(
    tab_ui: &mut TabUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Result<Option<WindowEvent>, ApplicationError> {
    match key_track.current_key().code {
        KeyCode::Tab => {
            if !in_editing_block(&mut tab_ui.prompt) {
                if tab_ui.prompt.is_status_insert() {
                    // change to normal prompt mode
                    return Ok(Some(tab_ui.set_prompt_window(false)));
                } else {
                    if tab_ui.prompt.text_buffer().is_empty() {
                        // change to response window
                        return Ok(Some(tab_ui.set_response_window()));
                    } else {
                        // send prompt
                        return Ok(Some(tab_ui.set_response_window()));
                    }
                }
            }
        }
        KeyCode::Enter => {
            // handle enter if not in editing mode
            if !tab_ui.prompt.is_status_insert() {
                let question = tab_ui.prompt.text_buffer().to_string();
                return Ok(Some(WindowEvent::Prompt(PromptAction::Write(
                    question,
                ))));
            }
        }
        KeyCode::Backspace => {
            if tab_ui.prompt.text_buffer().is_empty() {
                return Ok(Some(tab_ui.set_prompt_window(false)));
            }
            if !tab_ui.prompt.is_status_insert() {
                // change to insert mode
                tab_ui.prompt.set_status_insert();
            }
        }
        KeyCode::Esc => {
            // ensure blocks are closed if inside editing block
            if tab_ui.prompt.is_status_insert() {
                ensure_closed_block(&mut tab_ui.prompt)?;
            }
            return Ok(Some(tab_ui.set_prompt_window(false)));
        }
        KeyCode::Char(key) => {
            // catch Ctrl + shortcut key
            if key_track.current_key().modifiers == KeyModifiers::CONTROL {
                match key {
                    'c' => {
                        if tab_ui.prompt.text_buffer().is_empty() {
                            return Ok(Some(WindowEvent::Quit));
                        } else {
                            tab_ui.prompt.text_empty();
                        }
                    }
                    'q' => {
                        return Ok(Some(WindowEvent::Quit));
                    }
                    'a' => {
                        tab_ui.prompt.text_select_all();
                    }
                    'j' => {
                        tab_ui.prompt.text_insert_add("\n", None)?;
                    }
                    _ => {}
                }
                return Ok(Some(WindowEvent::PromptWindow(None)));
            } else if !tab_ui.prompt.is_status_insert() {
                // process regular key
                match key {
                    't' | 'T' => {
                        return Ok(Some(tab_ui.set_response_window()));
                    }
                    'i' | 'I' => {
                        return Ok(Some(tab_ui.set_prompt_window(true)));
                    }
                    '+' => {
                        tab_ui.set_primary_window(WindowKind::PromptWindow);
                    }
                    '-' => {
                        tab_ui.set_primary_window(WindowKind::ResponseWindow);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    handle_text_window_event(key_track, &mut tab_ui.prompt, is_running)
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
