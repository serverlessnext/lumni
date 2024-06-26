use std::error::Error;

use super::history::ChatHistory;
use super::prompt::Prompt;
use super::{
    ChatCompletionOptions, PromptOptions, DEFAULT_N_PREDICT,
    DEFAULT_TEMPERATURE, PERSONAS,
};

pub struct PromptInstruction {
    completion_options: ChatCompletionOptions,
    prompt_options: PromptOptions,
    system_prompt: SystemPrompt,
    prompt_template: Option<String>,
}

impl Default for PromptInstruction {
    fn default() -> Self {
        let completion_options = ChatCompletionOptions::default()
            .set_temperature(DEFAULT_TEMPERATURE)
            .set_n_predict(DEFAULT_N_PREDICT)
            .set_cache_prompt(true)
            .set_stream(true);

        PromptInstruction {
            completion_options,
            prompt_options: PromptOptions::default(),
            system_prompt: SystemPrompt::default(),
            prompt_template: None,
        }
    }
}

impl PromptInstruction {
    pub fn get_completion_options(&self) -> &ChatCompletionOptions {
        &self.completion_options
    }

    pub fn get_completion_options_mut(&mut self) -> &mut ChatCompletionOptions {
        &mut self.completion_options
    }

    pub fn get_prompt_options(&self) -> &PromptOptions {
        &self.prompt_options
    }

    pub fn get_prompt_options_mut(&mut self) -> &mut PromptOptions {
        &mut self.prompt_options
    }

    pub fn get_n_keep(&self) -> Option<usize> {
        self.completion_options.get_n_keep()
    }

    pub fn set_system_prompt(
        &mut self,
        instruction: String,
        token_length: Option<usize>,
    ) {
        self.system_prompt = SystemPrompt::new(instruction, token_length);
    }

    pub fn get_prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn get_instruction(&self) -> &str {
        self.system_prompt.get_instruction()
    }

    pub fn get_token_length(&self) -> Option<usize> {
        self.system_prompt.get_token_length()
    }

    pub fn preload_from_assistant(
        &mut self,
        assistant: String,
        history: &mut ChatHistory,
        instruction: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        // Find the selected persona by name
        let assistant_prompts: Vec<Prompt> = serde_yaml::from_str(PERSONAS)?;
        if let Some(prompt) = assistant_prompts
            .into_iter()
            .find(|p| p.name() == assistant)
        {
            // Set session instruction from persona's system prompt
            if let Some(instruction) = prompt.system_prompt() {
                self.set_system_prompt(instruction.to_string(), None);
            }
            // Load predefined exchanges from persona if available
            if let Some(exchanges) = prompt.exchanges() {
                *history = ChatHistory::new_with_exchanges(exchanges.clone());
            }

            if let Some(prompt_template) = prompt.prompt_template() {
                self.prompt_template = Some(prompt_template.to_string());
            }
            Ok(())
        } else {
            return Err(format!("Assistant '{}' not found in the dataset", assistant).into());
        }
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
