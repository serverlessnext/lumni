use std::error::Error;

use super::{Endpoints, PromptModelTrait, PromptOptions};

#[derive(Clone)]
pub struct Generic {
    prompt_options: PromptOptions,
    stop_tokens: Vec<String>,
}

impl Generic {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Generic {
            prompt_options: PromptOptions::new(),
            stop_tokens: vec![
                "### User: ".to_string(),
                "### Human: ".to_string(),
                "User: ".to_string(),
                "Human: ".to_string(),
            ],
        })
    }
}

impl PromptModelTrait for Generic {
    fn get_prompt_options(&self) -> &PromptOptions {
        &self.prompt_options
    }

    fn get_stop_tokens(&self) -> &Vec<String> {
        &self.stop_tokens
    }

    fn update_options_from_json(&mut self, json: &str) {
        self.prompt_options.update_from_json(json);
    }

    fn set_context_size(&mut self, context_size: usize) {
        self.prompt_options.set_context_size(context_size);
    }
}
