use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};
use lumni::api::error::ApplicationError;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    AppUi, ConversationWindowEvent, NavigationMode, TextWindowTrait,
    WindowEvent, WindowKind,
};
pub use crate::external as lumni;

pub fn handle_response_window_event(
    app_ui: &mut AppUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Result<WindowEvent, ApplicationError> {
    let conv_ui = match &mut app_ui.selected_mode {
        NavigationMode::Conversation(ui) => ui,
        _ => {
            return Err(ApplicationError::InvalidState(
                "Cant use response window. Not in Conversation mode"
                    .to_string(),
            ))
        }
    };

    match key_track.current_key().code {
        KeyCode::Down => {
            let (_, row) = conv_ui.response.get_column_row();
            if row == conv_ui.response.max_row_idx() {
                // jump from response window to prompt window
                return Ok(conv_ui.set_prompt_window(true));
            }
        }
        KeyCode::Tab => {
            return Ok(conv_ui.set_prompt_window(false));
        }
        KeyCode::Char(key) => {
            // catch Ctrl + shortcut key
            if key_track.current_key().modifiers == KeyModifiers::CONTROL {
                match key {
                    'c' => {
                        return Ok(WindowEvent::Quit);
                    }
                    'q' => {
                        return Ok(WindowEvent::Quit);
                    }
                    'a' => {
                        conv_ui.response.text_select_all();
                    }
                    _ => {}
                }
                return Ok(WindowEvent::Conversation(
                    ConversationWindowEvent::Response,
                ));
            } else {
                // process regular key
                match key {
                    'i' | 'I' => {
                        return Ok(conv_ui.set_prompt_window(true));
                    }
                    't' | 'T' => {
                        return Ok(conv_ui.set_prompt_window(false));
                    }
                    '+' => {
                        conv_ui.set_primary_window(WindowKind::ResponseWindow);
                    }
                    '-' => {
                        conv_ui.set_primary_window(WindowKind::EditorWindow);
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
    handle_text_window_event(key_track, &mut conv_ui.response, is_running)
}
