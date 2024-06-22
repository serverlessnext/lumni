use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    AppUi, LineType, PromptAction, PromptWindow, TextWindowTrait, WindowEvent,
};

pub fn handle_prompt_window_event(
    key_track: &mut KeyTrack,
    app_ui: &mut AppUi,
    is_running: Arc<AtomicBool>,
) -> WindowEvent {
    // catch Ctrl + shortcut key
    let mut prompt_window = &mut app_ui.prompt;

    if key_track.current_key().modifiers == KeyModifiers::CONTROL {
        match key_track.current_key().code {
            KeyCode::Char('c') => {
                if prompt_window.text_buffer().is_empty() {
                    return WindowEvent::Quit;
                } else {
                    prompt_window.text_empty();
                }
            }
            KeyCode::Char('q') => {
                return WindowEvent::Quit;
            }
            KeyCode::Char('a') => {
                prompt_window.text_select_all();
            }
            KeyCode::Char('j') => {
                prompt_window.text_insert_add("\n", None);
            }
            _ => {}
        }
        return WindowEvent::PromptWindow;
    } else {
        match key_track.current_key().code {
            KeyCode::Enter => {
                // send prompt if not inside editing block
                if !prompt_window.is_status_insert()
                    || !in_editing_block(prompt_window)
                {
                    ensure_closed_block(prompt_window);
                    let question = prompt_window.text_buffer().to_string();
                    prompt_window.text_empty();
                    return WindowEvent::Prompt(PromptAction::Write(question));
                }
            }
            KeyCode::Esc => {
                if prompt_window.is_status_insert() {
                    ensure_closed_block(prompt_window);
                }
            }
            KeyCode::Tab => {
                // TODO: tab inside
            }
            _ => {}
        }
    }
    handle_text_window_event(key_track, prompt_window, is_running)
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
