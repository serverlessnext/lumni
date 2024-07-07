use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::apps::builtin::llm::prompt::src::tui::WindowKind;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    LineType, PromptAction, PromptWindow, TabUi, TextWindowTrait, WindowEvent,
};

pub fn handle_prompt_window_event(
    tab_ui: &mut TabUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Option<WindowEvent> {
    match key_track.current_key().code {
        KeyCode::Tab => {
            if !in_editing_block(&mut tab_ui.prompt) {
                if tab_ui.prompt.is_status_insert() {
                    // change to normal prompt mode
                    return Some(tab_ui.set_prompt_window(false));
                } else {
                    if tab_ui.prompt.text_buffer().is_empty() {
                        // change to response window
                        return Some(tab_ui.set_response_window());
                    } else {
                        // send prompt
                        return Some(tab_ui.set_response_window());
                    }
                }
            }
        }
        KeyCode::Enter => {
            // handle enter if not in editing mode
            if !tab_ui.prompt.is_status_insert() {
                let question = tab_ui.prompt.text_buffer().to_string();
                return Some(WindowEvent::Prompt(PromptAction::Write(
                    question,
                )));
            }
        }
        KeyCode::Backspace => {
            if tab_ui.prompt.text_buffer().is_empty() {
                return Some(tab_ui.set_prompt_window(false));
            }
            if !tab_ui.prompt.is_status_insert() {
                // change to insert mode
                tab_ui.prompt.set_status_insert();
            }
        }
        KeyCode::Esc => {
            // ensure blocks are closed if inside editing block
            if tab_ui.prompt.is_status_insert() {
                ensure_closed_block(&mut tab_ui.prompt);
            }
            return Some(tab_ui.set_prompt_window(false));
        }
        KeyCode::Char(key) => {
            // catch Ctrl + shortcut key
            if key_track.current_key().modifiers == KeyModifiers::CONTROL {
                match key {
                    'c' => {
                        if tab_ui.prompt.text_buffer().is_empty() {
                            return Some(WindowEvent::Quit);
                        } else {
                            tab_ui.prompt.text_empty();
                        }
                    }
                    'q' => {
                        return Some(WindowEvent::Quit);
                    }
                    'a' => {
                        tab_ui.prompt.text_select_all();
                    }
                    'j' => {
                        tab_ui.prompt.text_insert_add("\n", None);
                    }
                    _ => {}
                }
                return Some(WindowEvent::PromptWindow);
            } else if !tab_ui.prompt.is_status_insert() {
                // process regular key
                match key {
                    't' | 'T' => {
                        return Some(tab_ui.set_response_window());
                    }
                    'i' | 'I' => {
                        return Some(tab_ui.set_prompt_window(true));
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

fn ensure_closed_block(prompt_window: &mut PromptWindow) {
    if let Some(closed_block) = is_closed_block(prompt_window) {
        if !closed_block {
            // close block
            prompt_window.text_append_with_insert("```", None);
        }
    }
}

fn in_editing_block(prompt_window: &mut PromptWindow) -> bool {
    let line_type = prompt_window.current_line_type().unwrap_or(LineType::Text);
    match line_type {
        LineType::Code(block_line) => !block_line.is_end(),
        _ => false,
    }
}
