use std::error::Error;

use super::{ModelData, PromptModelTrait};

#[derive(Clone, Debug)]
pub struct Generic {
    model_data: ModelData,
    stop_tokens: Vec<String>,
}

impl Generic {
    pub fn new(
        model_name: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Generic {
            model_data: ModelData::new(model_name),
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
    fn get_model_data(&self) -> &ModelData {
        &self.model_data
    }

    fn get_stop_tokens(&self) -> &Vec<String> {
        &self.stop_tokens
    }
}
