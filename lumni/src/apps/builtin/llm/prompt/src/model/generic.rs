use std::error::Error;

use super::PromptModelTrait;

#[derive(Clone, Debug)]
pub struct Generic {
    name: String,
    stop_tokens: Vec<String>,
}

impl Generic {
    pub fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Generic {
            name: name.to_string(),
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
