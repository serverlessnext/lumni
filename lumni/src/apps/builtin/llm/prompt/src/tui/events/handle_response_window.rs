use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{TabUi, PromptAction, TextWindowTrait, WindowEvent};

pub fn handle_response_window_event(
    tab_ui: &mut TabUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Option<WindowEvent> {
    if key_track.current_key().modifiers == KeyModifiers::CONTROL {
        // catch Ctrl + shortcut key
        match key_track.current_key().code {
            KeyCode::Char('c') => {
                if tab_ui.response.text_buffer().is_empty() {
                    return Some(WindowEvent::Quit);
                } else {
                    return Some(WindowEvent::Prompt(PromptAction::Clear));
                }
            }
            KeyCode::Char('q') => {
                return Some(WindowEvent::Quit);
            }
            KeyCode::Char('a') => {
                tab_ui.response.text_select_all();
            }
            _ => {}
        }
        return Some(WindowEvent::ResponseWindow);
    }
    handle_text_window_event(key_track, &mut tab_ui.response, is_running)
}
