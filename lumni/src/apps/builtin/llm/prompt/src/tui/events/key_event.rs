use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::text_window_event::handle_text_window_event;
use super::{
    AppUi, CommandLine, LineType, PromptAction, PromptWindow, ResponseWindow,
    TextWindowTrait, WindowEvent,
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

        let command_line = &mut app_ui.command_line;
        let prompt_window = &mut app_ui.prompt;
        let response_window = &mut app_ui.response;

        eprintln!("KeyEvent={:?}", key_event);
        // try to catch Shift+Enter key press in prompt window
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
                                "stop" => {
                                    return WindowEvent::Prompt(
                                        PromptAction::Stop,
                                    );
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
            WindowEvent::ResponseWindow => {
                // catch Ctrl + shortcut key
                if self.key_track.current_key().modifiers
                    == KeyModifiers::CONTROL
                {
                    match self.key_track.current_key().code {
                        KeyCode::Char('c') => {
                            if response_window.text_buffer().is_empty() {
                                return WindowEvent::Quit;
                            } else {
                                return WindowEvent::Prompt(
                                    PromptAction::Clear,
                                );
                            }
                        }
                        KeyCode::Char('q') => {
                            return WindowEvent::Quit;
                        }
                        KeyCode::Char('a') => {
                            response_window.text_select_all();
                        }
                        _ => {}
                    }
                    return WindowEvent::ResponseWindow;
                }
                handle_text_window_event(
                    &mut self.key_track,
                    response_window,
                    is_running,
                )
            }
            WindowEvent::PromptWindow => {
                // catch Ctrl + shortcut key
                if self.key_track.current_key().modifiers
                    == KeyModifiers::CONTROL
                {
                    match self.key_track.current_key().code {
                        KeyCode::Char('c') => {
                            if prompt_window.text_buffer().is_empty() {
                                return WindowEvent::Quit;
                            } else {
                                prompt_window.text_empty();
                            }
                        }
                        KeyCode::Char('q') => {
                            return WindowEvent::Quit;
                        }
                        KeyCode::Char('a') => {
                            prompt_window.text_select_all();
                        }
                        KeyCode::Char('j') => {
                            prompt_window.text_insert_add("\n", None);
                        }
                        _ => {}
                    }
                    return WindowEvent::PromptWindow;
                } else {
                    match self.key_track.current_key().code {
                        KeyCode::Enter => {
                            // send prompt if not inside editing block
                            if !prompt_window.is_status_insert()
                                || !in_editing_block(prompt_window)
                            {
                                ensure_closed_block(prompt_window);
                                let question =
                                    prompt_window.text_buffer().to_string();
                                prompt_window.text_empty();
                                return WindowEvent::Prompt(
                                    PromptAction::Write(question),
                                );
                            }
                        }
                        KeyCode::Esc => {
                            if prompt_window.is_status_insert() {
                                ensure_closed_block(prompt_window);
                            }
                        }
                        KeyCode::Tab => {
                            // TODO: tab inside
                        }
                        _ => {}
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

fn is_closed_block(prompt_window: &mut PromptWindow) -> Option<bool> {
    // return None if not inside a block
    // return Some(true) if block is closed, else return Some(false)
    let code_block = prompt_window.current_code_block();
    match code_block {
        Some(block) => Some(block.is_closed()),
        None => None,
    }
}

fn ensure_closed_block(prompt_window: &mut PromptWindow) {
    if let Some(closed_block) = is_closed_block(prompt_window) {
        if !closed_block {
            // close block
            prompt_window.text_append_with_insert("```", None);
        }
    }
}

fn in_editing_block(prompt_window: &mut PromptWindow) -> bool {
    let line_type = prompt_window.current_line_type().unwrap_or(LineType::Text);
    match line_type {
        LineType::Code(block_line) => !block_line.is_end(),
        _ => false,
    }
}
