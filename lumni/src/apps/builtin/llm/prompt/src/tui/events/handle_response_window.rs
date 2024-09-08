use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};
use lumni::api::error::ApplicationError;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    AppUi, ContentDisplayMode, ConversationEvent, TextWindowTrait, WindowKind,
    WindowMode,
};
pub use crate::external as lumni;

pub fn handle_response_window_event(
    app_ui: &mut AppUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Result<WindowMode, ApplicationError> {
    match key_track.current_key().code {
        KeyCode::Down => {
            let (_, row) = app_ui.conversation_ui.response.get_column_row();
            if row == app_ui.conversation_ui.response.max_row_idx() {
                // jump from response window to prompt window
                return Ok(app_ui.conversation_ui.set_prompt_window(true));
            }
        }
        KeyCode::Tab => {
            return Ok(app_ui.conversation_ui.set_prompt_window(false));
        }
        KeyCode::Char(key) => {
            // catch Ctrl + shortcut key
            if key_track.current_key().modifiers == KeyModifiers::CONTROL {
                match key {
                    'a' => {
                        app_ui.conversation_ui.response.text_select_all();
                    }
                    _ => {}
                }
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::Response,
                )));
            } else {
                // process regular key
                match key {
                    'i' | 'I' => {
                        return Ok(app_ui
                            .conversation_ui
                            .set_prompt_window(true));
                    }
                    't' | 'T' => {
                        return Ok(app_ui
                            .conversation_ui
                            .set_prompt_window(false));
                    }
                    '+' => {
                        app_ui
                            .conversation_ui
                            .set_primary_window(WindowKind::ResponseWindow);
                    }
                    '-' => {
                        app_ui
                            .conversation_ui
                            .set_primary_window(WindowKind::EditorWindow);
                    }
                    ' ' => {
                        if let Some(prev) = key_track.previous_key_str() {
                            if prev == " " {
                                // change to insert mode if double space
                                return Ok(app_ui
                                    .conversation_ui
                                    .set_prompt_window(true));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    handle_text_window_event(
        key_track,
        &mut app_ui.conversation_ui.response,
        is_running,
    )
}
