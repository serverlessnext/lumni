use std::error::Error;

use super::PromptModelTrait;

#[derive(Clone)]
pub struct Generic {
    stop_tokens: Vec<String>,
}

impl Generic {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Generic {
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
    fn get_stop_tokens(&self) -> &Vec<String> {
        &self.stop_tokens
    }
}
