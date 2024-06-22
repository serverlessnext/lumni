use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{AppUi, ModalWindow, PromptAction, TextWindowTrait, WindowEvent};

pub fn handle_command_line_event(
    key_track: &mut KeyTrack,
    app_ui: &mut AppUi,
    is_running: Arc<AtomicBool>,
) -> Option<WindowEvent> {
    let key_code = key_track.current_key().code;

    let command_line = &mut app_ui.command_line;
    let prompt_window = &mut app_ui.prompt;
    let response_window = &mut app_ui.response;

    match key_code {
        // Escape key
        KeyCode::Esc => {
            // exit command line
            command_line.text_empty();
            command_line.set_status_inactive();

            // switch to the active window
            if response_window.is_active() {
                response_window.set_status_normal();
                Some(WindowEvent::ResponseWindow)
            } else {
                prompt_window.set_status_normal();
                Some(WindowEvent::PromptWindow)
            }
        }
        KeyCode::Enter => {
            let command = command_line.text_buffer().to_string();
            command_line.text_empty();
            command_line.set_status_inactive();

            if command.starts_with(':') {
                match command.trim_start_matches(':') {
                    "q" => return Some(WindowEvent::Quit),
                    "w" => {
                        let question = prompt_window.text_buffer().to_string();
                        prompt_window.text_empty();
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
            command_line.text_empty();
            command_line.set_status_inactive();
            // TODO: instead of default, open a Config window
            Some(WindowEvent::Modal(Some(ModalWindow::default())))
        }
        _ => handle_text_window_event(key_track, command_line, is_running),
    }
}
