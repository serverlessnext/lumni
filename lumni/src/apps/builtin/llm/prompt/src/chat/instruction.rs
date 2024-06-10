use super::{
    ChatCompletionOptions, PromptOptions, DEFAULT_N_PREDICT,
    DEFAULT_TEMPERATURE,
};

pub struct PromptInstruction {
    completion_options: ChatCompletionOptions,
    prompt_options: PromptOptions,
    system_prompt: SystemPrompt,
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
