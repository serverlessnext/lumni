use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LlamaServerSystemPrompt {
    prompt: String,
    anti_prompt: String,
    assistant_name: String,
}

impl LlamaServerSystemPrompt {
    pub fn new(
        prompt: String,
        anti_prompt: String,
        assistant_name: String,
    ) -> Self {
        LlamaServerSystemPrompt {
            prompt,
            anti_prompt,
            assistant_name,
        }
    }
}

#[derive(Deserialize)]
pub struct LlamaServerDefaultGenerationSettings {
    n_ctx: usize,
}

#[derive(Deserialize)]
pub struct LlamaServerSettingsResponse {
    default_generation_settings: LlamaServerDefaultGenerationSettings,
}

impl LlamaServerSettingsResponse {
    pub fn get_n_ctx(&self) -> usize {
        self.default_generation_settings.n_ctx
    }
}

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
            temperature: None,
            top_k: None,
            top_p: None,
            n_keep: None,
            n_predict: None,
            cache_prompt: None,
            stop: None,
            stream: None,
        }
    }
}

impl ChatCompletionOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_from_json(&mut self, json: &str) {
        if let Ok(user_options) =
            serde_json::from_str::<ChatCompletionOptions>(json)
        {
            self.temperature = user_options.temperature.or(self.temperature);
            self.top_k = user_options.top_k.or(self.top_k);
            self.top_p = user_options.top_p.or(self.top_p);
            self.n_keep = user_options.n_keep.or(self.n_keep);
            self.n_predict = user_options.n_predict.or(self.n_predict);
            self.cache_prompt = user_options.cache_prompt.or(self.cache_prompt);
            self.stop = user_options.stop.or_else(|| self.stop.clone());
            self.stream = user_options.stream.or(self.stream);
        } else {
            log::warn!(
                "Failed to parse server chat options from JSON: {}",
                json
            );
        }
    }

    pub fn set_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn set_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    pub fn set_top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn set_n_keep(&mut self, n_keep: usize) -> &mut Self {
        self.n_keep = Some(n_keep);
        self
    }

    pub fn set_n_predict(mut self, n_predict: u32) -> Self {
        self.n_predict = Some(n_predict);
        self
    }

    pub fn set_cache_prompt(mut self, cache_prompt: bool) -> Self {
        self.cache_prompt = Some(cache_prompt);
        self
    }

    pub fn set_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn set_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PromptOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    n_ctx: Option<usize>,
}

impl Default for PromptOptions {
    fn default() -> Self {
        PromptOptions { n_ctx: None }
    }
}

impl PromptOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_from_json(&mut self, json: &str) {
        if let Ok(user_options) = serde_json::from_str::<PromptOptions>(json) {
            self.n_ctx = user_options.n_ctx.or(self.n_ctx);
        } else {
            log::warn!(
                "Failed to parse client chat options from JSON: {}",
                json
            );
        }
    }

    pub fn get_context_size(&self) -> Option<usize> {
        self.n_ctx
    }

    pub fn set_context_size(&mut self, context_size: usize) -> &mut Self {
        self.n_ctx = Some(context_size);
        self
    }
}
