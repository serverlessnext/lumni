
use super::{PromptModel, ChatOptions};

pub struct Llama3 {
    options: ChatOptions,
}

impl Llama3 {
    pub fn new() -> Self {
        Llama3 {
            options: ChatOptions::new()
                .set_temperature(0.2)
                .set_top_k(40)
                .set_top_p(0.9)
                .set_n_predict(8192)
                .set_cache_prompt(true)
                .set_stop(vec!["<|end_of_text|>".to_string(), "<|eot_id|>".to_string()])
                .set_stream(true),
        }
    }
}

impl PromptModel for Llama3 {
    fn update_options_from_json(&mut self, json: &str) {
        self.options.update_from_json(json);
    }

    fn set_n_keep(&mut self, n_keep: usize) {
        self.options.set_n_keep(n_keep);
    }

    fn chat_options(&self) -> &ChatOptions {
        &self.options
    }

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        if let Some(instruction) = instruction {
            return format!(
                "<|begin_of_text|>{}",
                self.fmt_prompt_message("system", instruction)
            ).to_string()
        } else {
            return "<|begin_of_text|>".to_string()
        }
    }

    fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        format!(
            "<|start_header_id|>{}<|end_header_id|>{}\n<|eot_id|>\n",
            role, message
        )
    }
}
