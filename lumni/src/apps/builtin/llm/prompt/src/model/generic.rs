use std::error::Error;

use super::defaults::*;
use super::{
    ChatCompletionOptions, Endpoints, PromptModelTrait, PromptOptions,
};

pub struct Generic {
    prompt_options: PromptOptions,
    completion_options: ChatCompletionOptions,
    endpoints: Endpoints,
}

impl Generic {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Generic {
            prompt_options: PromptOptions::new(),
            completion_options: ChatCompletionOptions::new()
                .set_temperature(DEFAULT_TEMPERATURE)
                .set_n_predict(DEFAULT_N_PREDICT)
                .set_cache_prompt(true)
                .set_stop(vec![
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

impl PromptModelTrait for Generic {
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
}