use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;
use lumni::api::error::ApplicationError;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    AppUi, ContentDisplayMode, ConversationEvent, ModalEvent, ModalWindowType,
    PromptAction, TextWindowTrait, WindowMode,
};
pub use crate::external as lumni;

pub fn handle_command_line_event(
    app_ui: &mut AppUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Result<WindowMode, ApplicationError> {
    let key_code = key_track.current_key().code;
    match key_code {
        KeyCode::Esc => {
            // exit command line
            app_ui.command_line.text_empty();
            app_ui.command_line.set_status_inactive();

            Ok(app_ui.set_response_window())
        }
        KeyCode::Enter => {
            let command = app_ui.command_line.text_buffer().to_string();
            app_ui.command_line.text_empty();
            app_ui.command_line.set_status_inactive();
            if command.starts_with(':') {
                match command.trim_start_matches(':') {
                    "w" => {
                        let question = app_ui
                            .conversation_ui
                            .prompt
                            .text_buffer()
                            .to_string();
                        app_ui.conversation_ui.prompt.text_empty();
                        return Ok(WindowMode::Prompt(PromptAction::Write(
                            question,
                        )));
                    }
                    "stop" => {
                        return Ok(WindowMode::Prompt(PromptAction::Stop));
                    }
                    _ => {} // command not recognized
                }
            }
            Ok(WindowMode::Conversation(Some(
                ConversationEvent::PromptRead,
            )))
        }
        KeyCode::Char(':') => {
            // double-colon opens Modal (Config) window
            app_ui.command_line.text_empty();
            app_ui.command_line.set_status_inactive();
            Ok(WindowMode::Modal(ModalEvent::Open(
                ModalWindowType::ProfileEdit,
            )))
        }
        _ => handle_text_window_event(
            key_track,
            &mut app_ui.command_line,
            is_running,
        ),
    }
}
