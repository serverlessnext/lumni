use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatOptions {
    temperature: f64,
    top_k: u32,
    top_p: f64,
    n_keep: usize,
    n_predict: u32,
    cache_prompt: bool,
    stop: Vec<String>,
    stream: bool,
}

impl Default for ChatOptions {
    fn default() -> Self {
        ChatOptions {
            temperature: 0.2,
            top_k: 40,
            top_p: 0.9,
            n_keep: 0,
            n_predict: 512,
            cache_prompt: true,
            // based on llama3
            stop: vec!["<|end_of_text|>".to_string(), "<|eot_id|>".to_string()],
            stream: true,
        }
    }
}

impl ChatOptions {
    pub fn new_from_args(options: Option<&String>) -> ChatOptions {
        match options {
            Some(options_str) => {
                serde_json::from_str(options_str).unwrap_or_default()
            }
            None => ChatOptions::default(),
        }
    }

    pub fn temperature(&self) -> f64 {
        self.temperature
    }

    pub fn top_k(&self) -> u32 {
        self.top_k
    }

    pub fn top_p(&self) -> f64 {
        self.top_p
    }

    pub fn n_keep(&self) -> usize {
        self.n_keep
    }

    pub fn n_predict(&self) -> u32 {
        self.n_predict
    }

    pub fn cache_prompt(&self) -> bool {
        self.cache_prompt
    }

    pub fn stop(&self) -> &Vec<String> {
        &self.stop
    }

    pub fn stream(&self) -> bool {
        self.stream
    }
}
