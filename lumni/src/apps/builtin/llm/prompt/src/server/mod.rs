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
pub use super::defaults::*;
pub use super::model::{PromptModelTrait, PromptRole};
use crate::external as lumni;

pub enum ModelServer {
    Llama(Llama),
}

impl ModelServer {
    pub fn from_str(
        s: &str,
        prompt_options: PromptOptions,
        completion_options: ChatCompletionOptions,
    ) -> Result<Self, Box<dyn Error>> {
        match s {
            "llama" => Ok(ModelServer::Llama(Llama::new(
                prompt_options,
                completion_options,
            )?)),
            _ => Ok(ModelServer::Llama(Llama::new(
                prompt_options,
                completion_options,
            )?)),
        }
    }
}

#[async_trait]
impl ServerTrait for ModelServer {
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

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        model: &Box<dyn PromptModelTrait>,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            ModelServer::Llama(llama) => {
                llama.completion(exchanges, model, tx, cancel_rx).await
            }
        }
    }
}

#[async_trait]
pub trait ServerTrait: Send + Sync {
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

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        model: &Box<dyn PromptModelTrait>,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>>;
}
