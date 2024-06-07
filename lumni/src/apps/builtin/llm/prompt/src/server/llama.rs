use std::error::Error;

use super::defaults::*;
use super::{
    ServerTrait,
    PromptModelTrait,
    ChatCompletionOptions,
};

pub struct Llama {
    completion_options: ChatCompletionOptions,
}

impl Llama {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Llama {
            completion_options: ChatCompletionOptions::new()
                .set_temperature(DEFAULT_TEMPERATURE)
                .set_n_predict(DEFAULT_N_PREDICT)
                .set_cache_prompt(true)
                .set_stream(true),
        })
    }
}

impl ServerTrait for Llama {
    fn get_completion_options(&self) -> &ChatCompletionOptions {
        &self.completion_options
    }

    fn update_options_from_json(&mut self, json: &str) {
        self.completion_options.update_from_json(json);
    }

    fn update_options_from_model(&mut self, model: &dyn PromptModelTrait) {
        self.completion_options.update_from_model(model);
    }

    fn set_n_keep(&mut self, n_keep: usize) {
        self.completion_options.set_n_keep(n_keep);
    }
}