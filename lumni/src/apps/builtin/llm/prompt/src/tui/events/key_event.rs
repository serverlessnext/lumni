use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::text_window_event::handle_text_window_event;
use super::{
    CommandLine, PromptAction, PromptWindow, ResponseWindow, TextWindowTrait,
    WindowEvent,
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

    pub fn reset(&mut self) {
        self.previous_char = None;
        self.numeric_input = NumericInput::new();
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

    pub fn retrieve_and_reset_numeric_input(&mut self) -> usize {
        self.numeric_input.retrieve_and_reset()
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

    pub fn retrieve_and_reset(&mut self) -> usize {
        let num = self.last_confirmed_input.take().unwrap_or(1);
        self.last_confirmed_input = None; // Reset the stored value after retrieval.
        num
    }

    pub fn clear(&mut self) {
        self.buffer = None;
    }

    pub fn reset(&mut self) {
        self.last_confirmed_input = None;
        self.clear();
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
        command_line: &mut CommandLine<'_>,
        prompt_window: &mut PromptWindow<'_>,
        is_running: Arc<AtomicBool>,
        response_window: &mut ResponseWindow<'_>,
    ) -> WindowEvent {
        self.key_track.update_key(key_event);

        match current_mode {
            WindowEvent::CommandLine(_) => {
                let key_code = self.key_track.current_key().code;

                match key_code {
                    // Escape key
                    KeyCode::Esc => {
                        // exit command line
                        command_line.text_empty();
                        command_line.set_status_inactive();

                        // switch to the active window
                        if response_window.is_active() {
                            response_window.set_status_normal();
                            WindowEvent::ResponseWindow
                        } else {
                            prompt_window.set_status_normal();
                            WindowEvent::PromptWindow
                        }
                    }
                    KeyCode::Enter => {
                        let command = command_line.text_buffer().to_string();
                        command_line.text_empty();
                        command_line.set_status_inactive();

                        if command.starts_with(':') {
                            match command.trim_start_matches(':') {
                                "q" => return WindowEvent::Quit,
                                "w" => {
                                    let question =
                                        prompt_window.text_buffer().to_string();
                                    prompt_window.text_empty();
                                    return WindowEvent::Prompt(
                                        PromptAction::Write(question),
                                    );
                                }
                                "clear" => {
                                    return WindowEvent::Prompt(
                                        PromptAction::Clear,
                                    )
                                }
                                _ => {} // command not recognized
                            }
                        }
                        WindowEvent::PromptWindow
                    }
                    _ => handle_text_window_event(
                        &mut self.key_track,
                        command_line,
                        is_running,
                    ),
                }
            }
            WindowEvent::ResponseWindow => handle_text_window_event(
                &mut self.key_track,
                response_window,
                is_running,
            ),
            WindowEvent::PromptWindow => {
                // catch Enter key press in prompt window
                if self.key_track.current_key().code == KeyCode::Enter {
                    let question = prompt_window.text_buffer().to_string();
                    // send prompt if not editing, or if the last character is a space
                    if !prompt_window.is_status_insert()
                        || question.chars().last() == Some(' ')
                    {
                        prompt_window.text_empty();
                        return WindowEvent::Prompt(PromptAction::Write(
                            question,
                        ));
                    }
                }
                handle_text_window_event(
                    &mut self.key_track,
                    prompt_window,
                    is_running,
                )
            }
            _ => current_mode,
        }
    }
}