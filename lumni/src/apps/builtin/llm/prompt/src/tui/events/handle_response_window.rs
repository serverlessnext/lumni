use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};
use lumni::api::error::ApplicationError;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{TabUi, TextWindowTrait, WindowEvent, WindowKind};
pub use crate::external as lumni;

pub fn handle_response_window_event(
    tab_ui: &mut TabUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Result<Option<WindowEvent>, ApplicationError> {
    match key_track.current_key().code {
        KeyCode::Tab => {
            return Ok(Some(tab_ui.set_prompt_window(false)));
        }
        KeyCode::Char(key) => {
            // catch Ctrl + shortcut key
            if key_track.current_key().modifiers == KeyModifiers::CONTROL {
                match key {
                    'c' => {
                        return Ok(Some(WindowEvent::Quit));
                    }
                    'q' => {
                        return Ok(Some(WindowEvent::Quit));
                    }
                    'a' => {
                        tab_ui.response.text_select_all();
                    }
                    _ => {}
                }
                return Ok(Some(WindowEvent::ResponseWindow));
            } else {
                // process regular key
                match key {
                    'i' | 'I' => {
                        return Ok(Some(tab_ui.set_prompt_window(true)));
                    }
                    't' | 'T' => {
                        return Ok(Some(tab_ui.set_prompt_window(false)));
                    }
                    '+' => {
                        tab_ui.set_primary_window(WindowKind::ResponseWindow);
                    }
                    '-' => {
                        tab_ui.set_primary_window(WindowKind::PromptWindow);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    handle_text_window_event(key_track, &mut tab_ui.response, is_running)
}
