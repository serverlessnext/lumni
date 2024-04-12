
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

use tui_textarea::{Input, Key, TextArea};
use super::{PromptLog, TransitionAction};

pub struct CommandLine {
    prompt_log: PromptLog,
}

impl CommandLine {
    pub fn new() -> Self {
        let instruction = "A chat between a curious human and an artificial \
                           intelligence assistant. The assistant gives \
                           helpful, detailed, and polite answers to the \
                           human's questions."
            .to_string();
        let prompt_log = PromptLog::new(10, instruction);
        Self { prompt_log }
    }

    pub fn clear(&mut self, command_line: &mut TextArea<'_>) {
        command_line.select_all();
        command_line.cut();
    }

    pub async fn process_command(
        &mut self,
        command_line: &mut TextArea<'_>,
        prompt_edit: &TextArea<'_>,
    ) -> TransitionAction {
        let command = command_line.lines()[0].to_string();
        self.clear(command_line);

        if command.starts_with(':') {
            match command.trim_start_matches(':') {
                "q" => return TransitionAction::Quit,
                "w" => {
                    let question: String = prompt_edit.lines().join("\n");
                    return TransitionAction::WritePrompt(question);
                }
                _ => {} // Handle other commands as needed
            }
        }
        TransitionAction::EditPrompt
    }
}

pub async fn transition_command_line(
    cl: &mut CommandLine,
    command_line: &mut TextArea<'_>,
    prompt_edit: &mut TextArea<'_>,
    input: Input,
) -> TransitionAction {
    match input {
        Input { key: Key::Esc, .. } => {
            // catch esc key - clear command line
            cl.clear(command_line)
        }
        Input {
            key: Key::Enter, ..
        } => {
            // process command
            let response = cl.process_command(
                command_line,
                prompt_edit,
            ).await;
            cl.clear(prompt_edit);
            return response;
        }
        _ => {
            command_line.input(input.clone());
            // continue Command Line mode
            return TransitionAction::CommandLine;
        }
    };
    // exit command line mode
    return TransitionAction::EditPrompt;
}
