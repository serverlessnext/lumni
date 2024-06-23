use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    AppUi, ModalWindowType, PromptAction, TextWindowTrait, WindowEvent,
};

pub fn handle_command_line_event(
    app_ui: &mut AppUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Option<WindowEvent> {
    let key_code = key_track.current_key().code;

    match key_code {
        // Escape key
        KeyCode::Esc => {
            // exit command line
            app_ui.command_line.text_empty();
            app_ui.command_line.set_status_inactive();

            // switch to the active window
            if app_ui.response.is_active() {
                app_ui.response.set_status_normal();
                Some(WindowEvent::ResponseWindow)
            } else {
                app_ui.prompt.set_status_normal();
                Some(WindowEvent::PromptWindow)
            }
        }
        KeyCode::Enter => {
            let command = app_ui.command_line.text_buffer().to_string();
            app_ui.command_line.text_empty();
            app_ui.command_line.set_status_inactive();

            if command.starts_with(':') {
                match command.trim_start_matches(':') {
                    "q" => return Some(WindowEvent::Quit),
                    "w" => {
                        let question = app_ui.prompt.text_buffer().to_string();
                        app_ui.prompt.text_empty();
                        return Some(WindowEvent::Prompt(PromptAction::Write(
                            question,
                        )));
                    }
                    "clear" => {
                        return Some(WindowEvent::Prompt(PromptAction::Clear))
                    }
                    "stop" => {
                        return Some(WindowEvent::Prompt(PromptAction::Stop));
                    }
                    _ => {} // command not recognized
                }
            }
            Some(WindowEvent::PromptWindow)
        }
        KeyCode::Char(':') => {
            // double-colon opens Modal (Config) window
            app_ui.command_line.text_empty();
            app_ui.command_line.set_status_inactive();
            Some(WindowEvent::Modal(ModalWindowType::Config))
        }
        _ => handle_text_window_event(
            key_track,
            &mut app_ui.command_line,
            is_running,
        ),
    }
}
