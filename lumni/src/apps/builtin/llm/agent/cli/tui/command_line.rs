use tui_textarea::{Input, Key, TextArea};

use super::{PromptLogWindow, PromptAction, TransitionAction};

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
        prompt_edit: &TextArea<'_>,
    ) -> TransitionAction {
        let command = command_line.lines()[0].to_string();
        self.clear(command_line);

        if command.starts_with(':') {
            match command.trim_start_matches(':') {
                "q" => return TransitionAction::Quit,
                "w" => {
                    let question: String = prompt_edit.lines().join("\n");
                    return TransitionAction::Prompt(PromptAction::Write(
                        question,
                    ));
                }
                "clear" => {
                    return TransitionAction::Prompt(PromptAction::Clear)
                }
                _ => {} // Handle other commands as needed
            }
        }
        TransitionAction::PromptWindow
    }
}

pub async fn transition_command_line(
    cl: &mut CommandLine,
    command_line: &mut TextArea<'_>,
    editor_window: &mut TextArea<'_>,
    response_window: &mut PromptLogWindow<'_>,
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
            let response = cl.process_command(command_line, editor_window).await;
            cl.clear(editor_window);
            return response;
        }
        _ => {
            command_line.input(input.clone());
            // continue Command Line mode
            return TransitionAction::CommandLine;
        }
    };
    // exit command line mode
    if response_window.is_active() {
        TransitionAction::ResponseWindow
    } else {
        TransitionAction::PromptWindow
    }
}
