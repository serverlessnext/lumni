use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::KeyCode;

use super::key_event::KeyTrack;
use super::text_window_event::handle_text_window_event;
use super::{
    ModalWindowType, PromptAction, TabUi, TextWindowTrait, WindowEvent,
};

pub fn handle_command_line_event(
    tab_ui: &mut TabUi,
    key_track: &mut KeyTrack,
    is_running: Arc<AtomicBool>,
) -> Option<WindowEvent> {
    let key_code = key_track.current_key().code;

    match key_code {
        // Escape key
        KeyCode::Esc => {
            // exit command line
            tab_ui.command_line.text_empty();
            tab_ui.command_line.set_status_inactive();

            // switch to the active window
            if tab_ui.response.is_active() {
                tab_ui.response.set_status_normal();
                Some(WindowEvent::ResponseWindow)
            } else {
                tab_ui.prompt.set_status_normal();
                Some(WindowEvent::PromptWindow)
            }
        }
        KeyCode::Enter => {
            let command = tab_ui.command_line.text_buffer().to_string();
            tab_ui.command_line.text_empty();
            tab_ui.command_line.set_status_inactive();

            if command.starts_with(':') {
                match command.trim_start_matches(':') {
                    "q" => return Some(WindowEvent::Quit),
                    "w" => {
                        let question = tab_ui.prompt.text_buffer().to_string();
                        tab_ui.prompt.text_empty();
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
            tab_ui.command_line.text_empty();
            tab_ui.command_line.set_status_inactive();
            Some(WindowEvent::Modal(ModalWindowType::Config))
        }
        _ => handle_text_window_event(
            key_track,
            &mut tab_ui.command_line,
            is_running,
        ),
    }
}
