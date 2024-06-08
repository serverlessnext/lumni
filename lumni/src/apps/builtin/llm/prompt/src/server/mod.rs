mod defaults;
mod endpoints;
mod llama;
mod options;

use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
pub use endpoints::Endpoints;
pub use llama::Llama;
pub use lumni::HttpClient;
pub use options::{ChatCompletionOptions, PromptOptions};
use tokio::sync::{mpsc, oneshot};

pub use super::chat::{
    http_get_with_response, http_post, ChatExchange, ChatHistory, TokenResponse,
};
pub use super::model::{PromptModelTrait, PromptRole};
use crate::{external as lumni, http};

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

#[async_trait]
impl ServerTrait for ModelServer {
    fn get_prompt_options(&self) -> &PromptOptions {
        match self {
            ModelServer::Llama(llama) => llama.get_prompt_options(),
        }
    }

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

    async fn put_system_prompt(
        &self,
        instruction: &str,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            ModelServer::Llama(llama) => {
                llama.put_system_prompt(instruction).await
            }
        }
    }

    async fn get_context_size(&mut self) -> Result<usize, Box<dyn Error>> {
        match self {
            ModelServer::Llama(llama) => llama.get_context_size().await,
        }
    }

    async fn tokenizer(
        &self,
        content: &str,
    ) -> Result<Option<TokenResponse>, Box<dyn Error>> {
        match self {
            ModelServer::Llama(llama) => llama.tokenizer(content).await,
        }
    }

    fn completion_api_payload(
        &self,
        model: &Box<dyn PromptModelTrait>,
        exchanges: &Vec<ChatExchange>,
    ) -> Result<String, serde_json::Error> {
        match self {
            ModelServer::Llama(llama) => {
                llama.completion_api_payload(model, exchanges)
            }
        }
    }
}

#[async_trait]
pub trait ServerTrait: Send + Sync {
    fn get_prompt_options(&self) -> &PromptOptions;
    fn get_completion_options(&self) -> &ChatCompletionOptions;
    fn get_endpoints(&self) -> &Endpoints;
    fn update_options_from_json(&mut self, json: &str);
    fn update_options_from_model(&mut self, model: &dyn PromptModelTrait);
    fn set_n_keep(&mut self, n_keep: usize);
    async fn put_system_prompt(
        &self,
        instruction: &str,
    ) -> Result<(), Box<dyn Error>>;
    async fn get_context_size(&mut self) -> Result<usize, Box<dyn Error>>;

    async fn tokenizer(
        &self,
        content: &str,
    ) -> Result<Option<TokenResponse>, Box<dyn Error>>;

    fn completion_api_payload(
        &self,
        model: &Box<dyn PromptModelTrait>,
        exchanges: &Vec<ChatExchange>,
    ) -> Result<String, serde_json::Error>;

    fn completion_endpoint(&self) -> Result<String, Box<dyn Error>> {
        self.get_endpoints()
            .get_completion()
            .map(|url| url.to_string())
            .ok_or_else(|| "Completion endpoint must be set".into())
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        model: &Box<dyn PromptModelTrait>,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>> {
        let data_payload = self.completion_api_payload(model, exchanges);

        let http_client = HttpClient::new();
        let completion_endpoint = self.completion_endpoint()?;
        if let Ok(payload) = data_payload {
            http_post(completion_endpoint, http_client, tx, payload, cancel_rx)
                .await;
        }
        Ok(())
    }
}
