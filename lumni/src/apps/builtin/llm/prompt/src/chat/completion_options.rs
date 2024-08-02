use serde::{Deserialize, Serialize};

use super::db::ModelServerName;
use super::{DEFAULT_N_PREDICT, DEFAULT_TEMPERATURE};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_keep: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_prompt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_server: Option<ModelServerName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_options: Option<AssistantOptions>,
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
            model_server: None,
            assistant_options: None,
        }
    }
}

#[allow(dead_code)]
impl ChatCompletionOptions {
    pub fn update(
        &mut self,
        value: serde_json::Value,
    ) -> Result<(), serde_json::Error> {
        let user_options =
            serde_json::from_value::<ChatCompletionOptions>(value)?;
        self.temperature = user_options.temperature.or(self.temperature);
        self.top_k = user_options.top_k.or(self.top_k);
        self.top_p = user_options.top_p.or(self.top_p);
        self.n_keep = user_options.n_keep.or(self.n_keep);
        self.n_predict = user_options.n_predict.or(self.n_predict);
        self.cache_prompt = user_options.cache_prompt.or(self.cache_prompt);
        self.stop = user_options.stop.or_else(|| self.stop.clone());
        self.stream = user_options.stream.or(self.stream);
        self.model_server =
            user_options.model_server.or(self.model_server.clone());
        self.assistant_options = user_options
            .assistant_options
            .or_else(|| self.assistant_options.clone());
        Ok(())
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

    pub fn set_assistant_options(&mut self, options: AssistantOptions) {
        self.assistant_options = Some(options);
    }

    pub fn get_assistant_options(&self) -> Option<&AssistantOptions> {
        self.assistant_options.as_ref()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssistantOptions {
    pub name: String,              // name of assistant used
    pub preloaded_messages: usize, // number of messages loaded by the assistant, does not include the first system message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
}
