use std::error::Error;

use url::Url;

use super::defaults::*;
use super::{ChatCompletionOptions, Endpoints, PromptModelTrait, ServerTrait};

pub const DEFAULT_TOKENIZER_ENDPOINT: &str = "http://localhost:8080/tokenize";
pub const DEFAULT_COMPLETION_ENDPOINT: &str =
    "http://localhost:8080/completion";
pub const DEFAULT_SETTINGS_ENDPOINT: &str = "http://localhost:8080/props";

pub struct Llama {
    endpoints: Endpoints,
    completion_options: ChatCompletionOptions,
}

impl Llama {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(DEFAULT_COMPLETION_ENDPOINT)?)
            .set_tokenizer(Url::parse(DEFAULT_TOKENIZER_ENDPOINT)?)
            .set_settings(Url::parse(DEFAULT_SETTINGS_ENDPOINT)?);

        Ok(Llama {
            endpoints,
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

    fn get_endpoints(&self) -> &Endpoints {
        &self.endpoints
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
