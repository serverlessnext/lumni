use super::{PromptModel, ChatOptions};

pub struct Generic {
    options: ChatOptions,
}

impl Generic {
    pub fn new() -> Self {
        Generic {
            options: ChatOptions::new()
                .set_temperature(0.2)
                .set_top_k(40)
                .set_top_p(0.9)
                .set_n_predict(8192)
                .set_cache_prompt(true)
                .set_stop(vec!["### Human: ".to_string(), "Human: ".to_string()])
                .set_stream(true),
        }
    }
}

impl PromptModel for Generic {
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
            return format!("{}\n", instruction).to_string()
        } else {
            return "".to_string()
        }
    }

    fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        match role {
            "user" => format!("### Human: {}", message),
            "assistant" => format!("### Assistant: {}", message),
            _ => format!("### {}: {}", role, message), // Default case for any other role
        }
    }
}
