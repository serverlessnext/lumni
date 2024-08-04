use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::handle_command_line::handle_command_line_event;
use super::handle_prompt_window::handle_prompt_window_event;
use super::handle_response_window::handle_response_window_event;
use super::{
    AppUi, ApplicationError, ConversationDbHandler, ThreadedChatSession,
    WindowEvent,
};

#[derive(Debug, Clone)]
pub struct KeyTrack {
    previous_key_str: Option<String>,
    numeric_input: NumericInput,
    current_key: KeyEvent,
    leader_key_set: bool,
}

impl KeyTrack {
    pub fn new() -> Self {
        KeyTrack {
            previous_key_str: None,
            numeric_input: NumericInput::new(),
            current_key: KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
            leader_key_set: false,
        }
    }

    pub fn previous_key_str(&self) -> Option<&str> {
        self.previous_key_str.as_deref()
    }

    pub fn current_key(&self) -> KeyEvent {
        self.current_key
    }

    pub fn update_previous_key(&mut self, key_event: KeyEvent) {
        if let KeyCode::Char(c) = self.current_key.code {
            // copy previous key_event to previous_char
            self.previous_key_str = Some(c.to_string());
        } else {
            self.previous_key_str = None;
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

    pub fn update_previous_key_with_leader(
        &mut self,
        key_event: KeyEvent,
    ) -> Option<&str> {
        if let KeyCode::Char(new_c) = key_event.code {
            // Ensure previous_key_str is initialized to Some if it was None
            // Reset the previous key to an empty string if the current key is a space
            if self.previous_key_str.is_none() {
                self.previous_key_str = Some(String::new());
            }
            match new_c {
                ' ' => {
                    // double space
                    self.set_leader_key(false);
                }
                'i' => {
                    // currently insert always disables leader key
                    // this means we cant use <leader> + something that "i" to trigger an action
                    // may need to change this in the future after UI shows feedback that
                    // <leader> is enabled (e.g. with a popup to show matching commands)
                    self.set_leader_key(false);
                }
                _ => {
                    // append the current key to the previous key str
                    if let Some(prev_str) = &mut self.previous_key_str {
                        prev_str.push(new_c);
                    }
                }
            }
        } else {
            // non char key
            self.set_leader_key(false);
        }
        self.previous_key_str()
    }

    pub fn take_numeric_input(&mut self) -> Option<usize> {
        self.numeric_input.take()
    }

    pub fn leader_key_set(&self) -> bool {
        self.leader_key_set
    }

    pub fn set_leader_key(&mut self, leader_key_set: bool) {
        self.leader_key_set = leader_key_set;
        self.previous_key_str = None;
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
        tab_chat: &mut ThreadedChatSession,
        current_mode: WindowEvent,
        is_running: Arc<AtomicBool>,
        handler: &mut ConversationDbHandler,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        if !self.key_track.leader_key_set()
            || self
                .key_track
                .update_previous_key_with_leader(key_event)
                .is_none()
        {
            // leader key not set or updating leader is un-successful
            self.key_track.update_previous_key(key_event);
        }

        // try to catch Shift+Enter key press in prompt window
        match current_mode {
            WindowEvent::CommandLine(_) => handle_command_line_event(
                app_ui,
                &mut self.key_track,
                is_running,
            ),
            WindowEvent::ResponseWindow => handle_response_window_event(
                app_ui,
                &mut self.key_track,
                is_running,
            ),
            WindowEvent::PromptWindow(_) => handle_prompt_window_event(
                app_ui,
                &mut self.key_track,
                is_running,
            ),
            WindowEvent::Modal(window_type) => {
                // key event is handled by modal window
                if let Some(modal) = app_ui.modal.as_mut() {
                    let new_window_event = match modal
                        .handle_key_event(
                            &mut self.key_track,
                            tab_chat,
                            handler,
                        )
                        .await
                    {
                        Ok(Some(WindowEvent::Modal(next_window_type))) => {
                            if next_window_type == window_type {
                                // window remains un-changed
                                return Ok(Some(WindowEvent::Modal(
                                    window_type,
                                )));
                            }
                            WindowEvent::Modal(next_window_type)
                        }
                        Ok(Some(new_window_event)) => new_window_event,
                        Ok(None) => WindowEvent::PromptWindow(None), // default
                        Err(modal_error) => {
                            match modal_error {
                                ApplicationError::NotReady(message) => {
                                    // pass as warning to the user
                                    log::debug!("Not ready: {:?}", message);
                                    app_ui.command_line.set_alert(&format!(
                                        "Not Ready: {}",
                                        message
                                    ))?;
                                    return Ok(Some(WindowEvent::Modal(
                                        window_type,
                                    )));
                                }
                                _ => {
                                    log::error!(
                                        "Unexpected modal error: {:?}",
                                        modal_error
                                    );
                                    return Err(modal_error);
                                }
                            }
                        }
                    };
                    return Ok(Some(new_window_event));
                } else {
                    Ok(Some(WindowEvent::Modal(window_type)))
                }
            }
            _ => Ok(Some(current_mode)),
        }
    }
}
