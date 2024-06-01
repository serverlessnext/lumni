use std::error::Error;

use super::{ChatCompletionOptions, Endpoints, PromptModel, PromptOptions};

pub struct Llama3 {
    prompt_options: PromptOptions,
    completion_options: ChatCompletionOptions,
    endpoints: Endpoints,
}

impl Llama3 {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Llama3 {
            prompt_options: PromptOptions::new().set_context_size(8192),
            completion_options: ChatCompletionOptions::new()
                .set_temperature(0.2)
                .set_top_k(40)
                .set_top_p(0.9)
                .set_n_predict(8192)
                .set_cache_prompt(true)
                .set_stop(vec![
                    "<|end_of_text|>".to_string(),
                    "<|eot_id|>".to_string(),
                ])
                .set_stream(true),
            endpoints: Endpoints::default()?,
        })
    }
}

impl PromptModel for Llama3 {
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

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        if let Some(instruction) = instruction {
            return format!(
                "<|begin_of_text|>{}",
                self.fmt_prompt_message("system", instruction)
            )
            .to_string();
        } else {
            return "<|begin_of_text|>".to_string();
        }
    }

    fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        format!(
            "<|start_header_id|>{}<|end_header_id|>{}\n<|eot_id|>\n",
            role, message
        )
    }
}
