use std::error::Error;

use super::history::ChatHistory;
use super::prompt::Prompt;
use super::{
    ChatCompletionOptions, PromptOptions, ChatExchange,
    DEFAULT_N_PREDICT, DEFAULT_TEMPERATURE, PERSONAS,
};

pub struct PromptInstruction {
    completion_options: ChatCompletionOptions,
    prompt_options: PromptOptions,
    system_prompt: SystemPrompt,
    history: ChatHistory,
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
            history: ChatHistory::new(),
            prompt_template: None,
        }
    }
}

impl PromptInstruction {
    pub fn new(
        instruction: Option<String>,
        assistant: Option<String>,
        options: Option<&String>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut prompt_instruction = PromptInstruction::default();
        if let Some(json_str) = options {
            prompt_instruction
                .get_prompt_options_mut()
                .update_from_json(json_str);
            prompt_instruction
                .get_completion_options_mut()
                .update_from_json(json_str);
        }
    
        // If both instruction and assistant are None, use the default assistant
        let assistant = if instruction.is_none() && assistant.is_none() {
            // for useful responses, there should either be a system prompt or an
            // assistant set. If none are given use the default assistant.
            Some("Default".to_string())
        } else {
            assistant
        };
    
        if let Some(assistant) = assistant {
            prompt_instruction.preload_from_assistant(
                assistant,
                instruction, // add user-instruction with assistant
            )?;
        } else if let Some(instruction) = instruction {
            prompt_instruction.set_system_prompt(instruction);
        };
        Ok(prompt_instruction)
    }

    pub fn reset_history(&mut self) {
        self.history.reset();
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        self.history.update_last_exchange(answer);
    }

    pub fn get_last_exchange_mut(&mut self) -> Option<&mut ChatExchange> {
        self.history.get_last_exchange_mut()
    }
 
    pub fn new_prompt(
        &mut self,
        new_exchange: ChatExchange,
        max_token_length: usize,
        n_keep: Option<usize>,
    ) -> Vec<ChatExchange> {
        self.history.new_prompt(new_exchange, max_token_length, n_keep)
    }

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

    pub fn set_system_prompt(&mut self, instruction: String) {
        self.system_prompt = SystemPrompt::new(instruction);
    }

    pub fn get_system_token_length(&self) -> Option<usize> {
        self.system_prompt.get_token_length()
    }

    pub fn set_system_token_length(&mut self, token_length: Option<usize>) {
        self.system_prompt.set_token_length(token_length);
    }

    pub fn get_prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn get_instruction(&self) -> &str {
        self.system_prompt.get_instruction()
    }

    pub fn preload_from_assistant(
        &mut self,
        assistant: String,
        user_instruction: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        // Find the selected persona by name
        let assistant_prompts: Vec<Prompt> = serde_yaml::from_str(PERSONAS)?;
        if let Some(prompt) = assistant_prompts
            .into_iter()
            .find(|p| p.name() == assistant)
        {
            // system prompt is the assistant instruction + user instruction
            // default to empty string if either is not available
            let system_prompt =
                if let Some(assistant_instruction) = prompt.system_prompt() {
                    let system_prompt =
                        if let Some(user_instruction) = user_instruction {
                            // strip trailing whitespace from assistant instruction
                            format!(
                                "{} {}",
                                assistant_instruction.trim_end(),
                                user_instruction
                            )
                        } else {
                            assistant_instruction.to_string()
                        };
                    system_prompt
                } else {
                    user_instruction.unwrap_or_default()
                };
            self.set_system_prompt(system_prompt);

            // Load predefined exchanges from persona if available
            if let Some(exchanges) = prompt.exchanges() {
                self.history = ChatHistory::new_with_exchanges(exchanges.clone());
            }

            if let Some(prompt_template) = prompt.prompt_template() {
                self.prompt_template = Some(prompt_template.to_string());
            }
            Ok(())
        } else {
            return Err(format!(
                "Assistant '{}' not found in the dataset",
                assistant
            )
            .into());
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

    fn new(instruction: String) -> Self {
        SystemPrompt {
            instruction,
            token_length: None,
        }
    }

    fn get_instruction(&self) -> &str {
        &self.instruction
    }

    fn get_token_length(&self) -> Option<usize> {
        self.token_length
    }

    fn set_token_length(&mut self, token_length: Option<usize>) {
        self.token_length = token_length;
    }
}
