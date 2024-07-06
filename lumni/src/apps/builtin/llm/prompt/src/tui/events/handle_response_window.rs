use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{PromptAction, TabUi, TextWindowTrait, WindowEvent};

pub fn handle_response_window_event(
    tab_ui: &mut TabUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Option<WindowEvent> {
    match key_track.current_key().code {
        KeyCode::Tab => {
           return Some(tab_ui.set_prompt_window(false));
        }
        KeyCode::Char(key) => {
            // catch Ctrl + shortcut key
            if key_track.current_key().modifiers == KeyModifiers::CONTROL {
                match key {
                    'c' => {
                        if tab_ui.response.text_buffer().is_empty() {
                            return Some(WindowEvent::Quit);
                        } else {
                            return Some(WindowEvent::Prompt(PromptAction::Clear));
                        }
                    }
                    'q' => {
                        return Some(WindowEvent::Quit);
                    }
                    'a' => {
                        tab_ui.response.text_select_all();
                    }
                    _ => {}
                }
                return Some(WindowEvent::ResponseWindow);
            } else {
                // process regular key
                match key {
                    'i' | 'I' => {
                        return Some(tab_ui.set_prompt_window(true));
                    },
                    't' | 'T' => {
                        return Some(tab_ui.set_prompt_window(false));
                    },
                    _ => {}
                }
            }
        }
        _ => {}
    }
    handle_text_window_event(key_track, &mut tab_ui.response, is_running)
}
