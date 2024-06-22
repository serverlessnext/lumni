use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::handle_command_line::handle_command_line_event;
use super::handle_prompt_window::handle_prompt_window_event;
use super::handle_response_window::handle_response_window_event;
use super::{
    AppUi, WindowEvent,
};

#[derive(Debug, Clone)]
pub struct KeyTrack {
    previous_char: Option<String>,
    numeric_input: NumericInput,
    current_key: KeyEvent,
}

impl KeyTrack {
    pub fn new() -> Self {
        KeyTrack {
            previous_char: None,
            numeric_input: NumericInput::new(),
            current_key: KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
        }
    }

    pub fn previous_char(&self) -> Option<&str> {
        self.previous_char.as_deref()
    }

    pub fn current_key(&self) -> KeyEvent {
        self.current_key
    }

    pub fn update_key(&mut self, key_event: KeyEvent) {
        if let KeyCode::Char(c) = self.current_key.code {
            // copy previous key_event to previous_char
            self.previous_char = Some(c.to_string());
        } else {
            self.previous_char = None;
        }

        // /update current key with the new key_event
        self.current_key = key_event;
        if let KeyCode::Char(c) = key_event.code {
            if c.is_ascii_digit() {
                self.numeric_input.append_digit(c);
            } else {
                self.numeric_input.save_numeric_input();
            }
        }
    }

    pub fn take_numeric_input(&mut self) -> Option<usize> {
        self.numeric_input.take()
    }
}

#[derive(Debug, Clone)]
pub struct NumericInput {
    buffer: Option<String>,
    last_confirmed_input: Option<usize>,
}

impl NumericInput {
    pub fn new() -> Self {
        NumericInput {
            buffer: None,
            last_confirmed_input: None,
        }
    }

    pub fn append_digit(&mut self, digit: char) {
        if let Some(buffer) = &mut self.buffer {
            buffer.push(digit);
        } else {
            self.buffer = Some(digit.to_string());
        }
    }

    pub fn save_numeric_input(&mut self) {
        if let Some(num_str) = &self.buffer {
            if let Ok(num) = num_str.parse::<usize>() {
                self.last_confirmed_input = Some(num);
            }
        }
        self.buffer = None; // Always clear the buffer after saving or attempting to save.
    }

    pub fn take(&mut self) -> Option<usize> {
        self.last_confirmed_input.take()
    }
}

#[derive(Debug, Clone)]
pub struct KeyEventHandler {
    pub key_track: KeyTrack,
}

impl KeyEventHandler {
    pub fn new() -> Self {
        KeyEventHandler {
            key_track: KeyTrack::new(),
        }
    }

    pub async fn process_key(
        &mut self,
        key_event: KeyEvent,
        app_ui: &mut AppUi<'_>,
        current_mode: WindowEvent,
        is_running: Arc<AtomicBool>,
    ) -> WindowEvent {
        self.key_track.update_key(key_event);

        // try to catch Shift+Enter key press in prompt window
        match current_mode {
            WindowEvent::CommandLine(_) => handle_command_line_event(
                &mut self.key_track,
                app_ui,
                is_running,
            ),
            WindowEvent::ResponseWindow => handle_response_window_event(
                &mut self.key_track,
                app_ui,
                is_running,
            ),
            WindowEvent::PromptWindow => handle_prompt_window_event(
                &mut self.key_track,
                app_ui,
                is_running,
            ),
            WindowEvent::Modal(modal) => {
                // get Escape key press to close modal window
                if self.key_track.current_key().code == KeyCode::Esc {
                    app_ui.clear_modal();
                    WindowEvent::PromptWindow
                } else {
                    WindowEvent::Modal(modal)
                }
            }
            _ => current_mode,
        }
    }
}