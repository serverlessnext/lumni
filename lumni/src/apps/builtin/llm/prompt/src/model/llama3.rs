use std::error::Error;

use super::defaults::*;
use super::{
    ChatCompletionOptions, Endpoints, PromptModelTrait, PromptOptions,
    PromptRole,
};

pub struct Llama3 {
    prompt_options: PromptOptions,
    completion_options: ChatCompletionOptions,
    endpoints: Endpoints,
}

impl Llama3 {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Llama3 {
            prompt_options: PromptOptions::new(),
            completion_options: ChatCompletionOptions::new()
                .set_temperature(DEFAULT_TEMPERATURE)
                .set_n_predict(DEFAULT_N_PREDICT)
                .set_cache_prompt(true)
                .set_stop(vec![
                    "<|eot_id|>".to_string(),
                    "<|end_of_text|>".to_string(),
                    "### User: ".to_string(),
                    "### Human: ".to_string(),
                    "User: ".to_string(),
                    "Human: ".to_string(),
                ])
                .set_stream(true),
            endpoints: Endpoints::default()?,
        })
    }
}

impl PromptModelTrait for Llama3 {
    fn get_prompt_options(&self) -> &PromptOptions {
        &self.prompt_options
    }

    fn get_completion_options(&self) -> &ChatCompletionOptions {
        &self.completion_options
    }

    fn get_endpoints(&self) -> &Endpoints {
        &self.endpoints
    }

    fn update_options_from_json(&mut self, json: &str) {
        self.completion_options.update_from_json(json);
        self.prompt_options.update_from_json(json);
    }

    fn set_n_keep(&mut self, n_keep: usize) {
        self.completion_options.set_n_keep(n_keep);
    }

    fn set_context_size(&mut self, context_size: usize) {
        self.prompt_options.set_context_size(context_size);
    }

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        if let Some(instruction) = instruction {
            return format!(
                "<|begin_of_text|>{}",
                self.fmt_prompt_message(PromptRole::System, instruction)
            )
            .to_string();
        } else {
            return "<|begin_of_text|>".to_string();
        }
    }

    fn fmt_prompt_message(
        &self,
        prompt_role: PromptRole,
        message: &str,
    ) -> String {
        let role_handle = match prompt_role {
            PromptRole::User => "user",
            PromptRole::Assistant => "assistant",
            PromptRole::System => "system",
        };
        let mut prompt_message = String::new();
        prompt_message.push_str(&format!(
            "<|start_header_id|>{}<|end_header_id|>\n{}{}",
            role_handle,
            self.get_role_prefix(prompt_role),
            message
        ));
        if !message.is_empty() {
            prompt_message.push_str("<|eot_id|>\n");
        }
        prompt_message
    }
}