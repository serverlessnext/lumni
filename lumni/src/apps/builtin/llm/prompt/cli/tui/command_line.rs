use tui_textarea::{Input, Key, TextArea};

use super::events::PromptAction;
use super::{PromptWindow, ResponseWindow, TextWindowTrait, WindowEvent};

pub struct CommandLine {}

impl CommandLine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn clear(&mut self, command_line: &mut TextArea<'_>) {
        command_line.select_all();
        command_line.cut();
    }

    pub async fn process_command(
        &mut self,
        command_line: &mut TextArea<'_>,
        prompt_window: &mut PromptWindow<'_>,
    ) -> WindowEvent {
        let command = command_line.lines()[0].to_string();
        self.clear(command_line);

        if command.starts_with(':') {
            match command.trim_start_matches(':') {
                "q" => return WindowEvent::Quit,
                "w" => {
                    let question = prompt_window.text_buffer().to_string();
                    return WindowEvent::Prompt(PromptAction::Write(question));
                }
                "clear" => return WindowEvent::Prompt(PromptAction::Clear),
                _ => {} // Handle other commands as needed
            }
        }
        WindowEvent::PromptWindow
    }
}

pub async fn transition_command_line(
    cl: &mut CommandLine,
    command_line: &mut TextArea<'_>,
    prompt_window: &mut PromptWindow<'_>,
    response_window: &mut ResponseWindow<'_>,
    input: Input,
) -> WindowEvent {
    match input {
        Input { key: Key::Esc, .. } => {
            // catch esc key - clear command line
            cl.clear(command_line)
        }
        Input {
            key: Key::Enter, ..
        } => {
            // process command
            let response =
                cl.process_command(command_line, prompt_window).await;
            prompt_window.text_empty();
            return response;
        }
        _ => {
            command_line.input(input.clone());
            // continue Command Line mode
            return WindowEvent::CommandLine;
        }
    };
    // exit command line mode
    if response_window.is_active() {
        // return to response window if its active
        WindowEvent::ResponseWindow
    } else {
        WindowEvent::PromptWindow
    }
}