mod defaults;
mod endpoints;
mod llama;

use std::error::Error;

use async_trait::async_trait;
pub use endpoints::Endpoints;
pub use llama::Llama;

pub use super::model::{ChatCompletionOptions, PromptModelTrait};

pub enum ModelServer {
    Llama(Llama),
}

impl ModelServer {
    pub fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
        match s {
            "llama" => Ok(ModelServer::Llama(Llama::new()?)),
            _ => Ok(ModelServer::Llama(Llama::new()?)),
        }
    }
}

impl ServerTrait for ModelServer {
    fn get_completion_options(&self) -> &ChatCompletionOptions {
        match self {
            ModelServer::Llama(llama) => llama.get_completion_options(),
        }
    }

    fn get_endpoints(&self) -> &Endpoints {
        match self {
            ModelServer::Llama(llama) => llama.get_endpoints(),
        }
    }

    fn update_options_from_json(&mut self, json: &str) {
        match self {
            ModelServer::Llama(llama) => llama.update_options_from_json(json),
        }
    }

    fn update_options_from_model(&mut self, model: &dyn PromptModelTrait) {
        match self {
            ModelServer::Llama(llama) => llama.update_options_from_model(model),
        }
    }

    fn set_n_keep(&mut self, n_keep: usize) {
        match self {
            ModelServer::Llama(llama) => llama.set_n_keep(n_keep),
        }
    }
}

#[async_trait]
pub trait ServerTrait: Send + Sync {
    fn get_completion_options(&self) -> &ChatCompletionOptions;
    fn get_endpoints(&self) -> &Endpoints;
    fn update_options_from_json(&mut self, json: &str);
    fn update_options_from_model(&mut self, model: &dyn PromptModelTrait);
    fn set_n_keep(&mut self, n_keep: usize);
}
