use serde::{Deserialize, Serialize};

use super::{LLMDefinition, DEFAULT_N_PREDICT, DEFAULT_TEMPERATURE};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n_keep: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_prompt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

impl Default for ChatCompletionOptions {
    fn default() -> Self {
        ChatCompletionOptions {
            temperature: Some(DEFAULT_TEMPERATURE),
            top_k: None,
            top_p: None,
            n_keep: None,
            n_predict: Some(DEFAULT_N_PREDICT),
            cache_prompt: Some(true),
            stop: None,
            stream: Some(true),
        }
    }
}

impl ChatCompletionOptions {
    pub fn update_from_json(
        &mut self,
        json: &str,
    ) -> Result<(), serde_json::Error> {
        let user_options = serde_json::from_str::<ChatCompletionOptions>(json)?;
        self.temperature = user_options.temperature.or(self.temperature);
        self.top_k = user_options.top_k.or(self.top_k);
        self.top_p = user_options.top_p.or(self.top_p);
        self.n_keep = user_options.n_keep.or(self.n_keep);
        self.n_predict = user_options.n_predict.or(self.n_predict);
        self.cache_prompt = user_options.cache_prompt.or(self.cache_prompt);
        self.stop = user_options.stop.or_else(|| self.stop.clone());
        self.stream = user_options.stream.or(self.stream);
        Ok(())
    }

    pub fn update_from_model(&mut self, model: &LLMDefinition) {
        if self.stop.is_none() {
            self.stop = Some(model.get_stop_tokens().clone());
        }
    }

    pub fn set_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn set_n_keep(&mut self, n_keep: usize) -> &mut Self {
        self.n_keep = Some(n_keep);
        self
    }

    pub fn get_n_keep(&self) -> Option<usize> {
        self.n_keep
    }

    pub fn set_n_predict(mut self, n_predict: u32) -> Self {
        self.n_predict = Some(n_predict);
        self
    }

    pub fn set_cache_prompt(mut self, cache_prompt: bool) -> Self {
        self.cache_prompt = Some(cache_prompt);
        self
    }

    pub fn set_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
}
