use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    AppUi, LineType, PromptAction, PromptWindow, TextWindowTrait, WindowEvent,
};

pub fn handle_prompt_window_event(
    app_ui: &mut AppUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Option<WindowEvent> {
    if key_track.current_key().modifiers == KeyModifiers::CONTROL {
        // catch Ctrl + shortcut key
        match key_track.current_key().code {
            KeyCode::Char('c') => {
                if app_ui.prompt.text_buffer().is_empty() {
                    return Some(WindowEvent::Quit);
                } else {
                    app_ui.prompt.text_empty();
                }
            }
            KeyCode::Char('q') => {
                return Some(WindowEvent::Quit);
            }
            KeyCode::Char('a') => {
                app_ui.prompt.text_select_all();
            }
            KeyCode::Char('j') => {
                app_ui.prompt.text_insert_add("\n", None);
            }
            _ => {}
        }
        return Some(WindowEvent::PromptWindow);
    } else {
        match key_track.current_key().code {
            KeyCode::Enter => {
                // send prompt if not inside editing block
                if !app_ui.prompt.is_status_insert()
                    || !in_editing_block(&mut app_ui.prompt)
                {
                    ensure_closed_block(&mut app_ui.prompt);
                    let question = app_ui.prompt.text_buffer().to_string();
                    app_ui.prompt.text_empty();
                    return Some(WindowEvent::Prompt(PromptAction::Write(
                        question,
                    )));
                }
            }
            KeyCode::Esc => {
                if app_ui.prompt.is_status_insert() {
                    ensure_closed_block(&mut app_ui.prompt);
                }
            }
            KeyCode::Tab => {
                // TODO: tab inside
            }
            _ => {}
        }
    }
    handle_text_window_event(key_track, &mut app_ui.prompt, is_running)
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
