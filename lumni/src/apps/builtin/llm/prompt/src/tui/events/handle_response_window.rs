use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{AppUi, PromptAction, TextWindowTrait, WindowEvent};

pub fn handle_response_window_event(
    key_track: &mut KeyTrack,
    app_ui: &mut AppUi,
    is_running: Arc<AtomicBool>,
) -> Option<WindowEvent> {
    let response_window = &mut app_ui.response;

    if key_track.current_key().modifiers == KeyModifiers::CONTROL {
        // catch Ctrl + shortcut key
        match key_track.current_key().code {
            KeyCode::Char('c') => {
                if response_window.text_buffer().is_empty() {
                    return Some(WindowEvent::Quit);
                } else {
                    return Some(WindowEvent::Prompt(PromptAction::Clear));
                }
            }
            KeyCode::Char('q') => {
                return Some(WindowEvent::Quit);
            }
            KeyCode::Char('a') => {
                response_window.text_select_all();
            }
            _ => {}
        }
        return Some(WindowEvent::ResponseWindow);
    }
    handle_text_window_event(key_track, response_window, is_running)
}
