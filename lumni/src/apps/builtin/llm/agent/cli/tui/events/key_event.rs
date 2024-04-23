use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bytes::Bytes;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use super::command_line::handle_command_line_event;
use super::prompt_window::handle_prompt_window_event;
use super::response_window::handle_response_window_event;
use super::{CommandLine, PromptLogWindow, TextAreaHandler, WindowEvent};

#[derive(Debug, Clone)]
pub struct KeyTrack {
    previous_char: Option<String>,
    numeric_input: Option<String>,
    current_key: KeyEvent,
}

impl KeyTrack {
    pub fn new() -> Self {
        KeyTrack {
            previous_char: None,
            numeric_input: None,
            current_key: KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
        }
    }

    pub fn previous_char(&self) -> Option<&str> {
        self.previous_char.as_deref()
    }

    pub fn numeric_input(&self) -> Option<&str> {
        self.numeric_input.as_deref()
    }

    pub fn current_key(&self) -> KeyEvent {
        self.current_key
    }

    pub fn reset(&mut self) {
        self.previous_char = None;
        self.numeric_input = None;
    }

    pub fn update_key(&mut self, key_event: KeyEvent) {
        if let KeyCode::Char(c) = self.current_key.code {
            // Only update the previous_char if the current key was a character
            self.previous_char = Some(c.to_string());
        } else {
            // Reset previous character if the current key isn't a character
            self.previous_char = None;
        }

        // Updates the key tracking state based on the current key event
        self.current_key = key_event;
        if let KeyCode::Char(c) = key_event.code {
            if c.is_ascii_digit() {
                // Track numeric input
                if c == '0' && self.numeric_input.is_none() {
                    // Ignore leading zeros
                } else {
                    // Append to or initialize numeric input
                    if let Some(ref mut num_input) = self.numeric_input {
                        num_input.push(c);
                    } else {
                        self.numeric_input = Some(c.to_string());
                    }
                }
            } else {
                self.numeric_input = None; // Reset numeric input
            }
        }
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
        current_mode: WindowEvent,
        command_line_handler: &mut CommandLine,
        command_line: &mut TextArea<'_>,
        editor_window: &mut TextAreaHandler,
        is_running: Arc<AtomicBool>,
        tx: mpsc::Sender<Bytes>,
        response_window: &mut PromptLogWindow<'_>,
    ) -> WindowEvent {
        self.key_track.update_key(key_event);

        match current_mode {
            WindowEvent::CommandLine => {
                handle_command_line_event(
                    &self.key_track,
                    command_line_handler,
                    response_window,
                    editor_window,
                    command_line,
                    tx,
                    is_running,
                )
                .await
            }
            WindowEvent::ResponseWindow => handle_response_window_event(
                &self.key_track,
                response_window,
                command_line,
                is_running,
            ),
            WindowEvent::PromptWindow => {
                handle_prompt_window_event(
                    &self.key_track,
                    response_window,
                    editor_window,
                    command_line,
                    tx,
                    is_running,
                )
                .await
            }
            _ => current_mode,
        }
    }
}
