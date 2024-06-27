use serde::Deserialize;

use super::{ModelFormatter, ModelFormatterTrait};

#[derive(Debug, Clone, Deserialize)]
pub struct LLMDefinition {
    name: String,
    size: Option<usize>,
    description: Option<String>,
    family: Option<String>,
}

impl LLMDefinition {
    pub fn new(name: String) -> Self {
        LLMDefinition {
            name,
            size: None,
            description: None,
            family: None,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_size(&mut self, size: usize) -> &mut Self {
        self.size = Some(size);
        self
    }

    pub fn set_description(&mut self, description: String) -> &mut Self {
        self.description = Some(description);
        self
    }

    pub fn set_family(&mut self, family: String) -> &mut Self {
        self.family = Some(family);
        self
    }

    pub fn get_formatter(&self) -> Box<dyn ModelFormatterTrait> {
        Box::new(ModelFormatter::from_str(&self.name))
    }

    pub fn get_stop_tokens(&self) -> Vec<String> {
        self.get_formatter().get_stop_tokens().to_vec()
    }
}
