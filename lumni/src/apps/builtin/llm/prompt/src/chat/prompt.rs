use serde::{Deserialize, Serialize};

use super::exchange::ChatExchange;

#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    name: String,
    system_prompt: Option<String>,
    prompt_template: Option<String>,
    exchanges: Option<Vec<ChatExchange>>,
}

impl Prompt {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn exchanges(&self) -> Option<&Vec<ChatExchange>> {
        self.exchanges.as_ref()
    }
}

pub struct PromptInstruction {
    system_prompt: SystemPrompt,
}

impl Default for PromptInstruction {
    fn default() -> Self {
        PromptInstruction {
            system_prompt: SystemPrompt::default(),
        }
    }
}

impl PromptInstruction {
    pub fn set_system_prompt(
        &mut self,
        instruction: &str,
        token_length: Option<usize>,
    ) {
        self.system_prompt =
            SystemPrompt::new(instruction.to_string(), token_length);
    }

    pub fn get_instruction(&self) -> &str {
        self.system_prompt.get_instruction()
    }

    pub fn get_token_length(&self) -> Option<usize> {
        self.system_prompt.get_token_length()
    }
}

struct SystemPrompt {
    instruction: String,
    token_length: Option<usize>,
}

impl SystemPrompt {
    pub fn default() -> Self {
        SystemPrompt {
            instruction: "".to_string(),
            token_length: Some(0),
        }
    }

    fn new(instruction: String, token_length: Option<usize>) -> Self {
        SystemPrompt {
            instruction,
            token_length,
        }
    }

    fn get_instruction(&self) -> &str {
        &self.instruction
    }

    fn get_token_length(&self) -> Option<usize> {
        self.token_length
    }
}
