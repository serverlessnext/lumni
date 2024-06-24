mod endpoints;
mod llama;
mod ollama;

use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
pub use endpoints::Endpoints;
pub use llama::Llama;
pub use lumni::HttpClient;
pub use ollama::Ollama;
use tokio::sync::{mpsc, oneshot};

pub use super::chat::{
    http_get_with_response, http_post, http_post_with_response,
    ChatCompletionOptions, ChatExchange, ChatHistory, ChatMessage,
    PromptInstruction, TokenResponse,
};
pub use super::defaults::*;
pub use super::model::{PromptModelTrait, PromptRole};
use crate::external as lumni;

pub const SUPPORTED_MODEL_ENDPOINTS: [&str; 2] = ["llama", "ollama"];

pub enum ModelServer {
    Llama(Llama),
    Ollama(Ollama),
}

impl ModelServer {
    pub fn from_str(
        s: &str,
        //instruction: PromptInstruction,
    ) -> Result<Self, Box<dyn Error>> {
        match s {
            "llama" => Ok(ModelServer::Llama(Llama::new()?)),
            "ollama" => Ok(ModelServer::Ollama(Ollama::new()?)),
            _ => Ok(ModelServer::Llama(Llama::new()?)),
        }
    }
}

#[async_trait]
impl ServerTrait for ModelServer {
    async fn initialize(
        &mut self,
        model: &Box<dyn PromptModelTrait>,
        prompt_instruction: &mut PromptInstruction,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            ModelServer::Llama(llama) => {
                llama.initialize(model, prompt_instruction).await
            }
            ModelServer::Ollama(ollama) => {
                ollama.initialize(model, prompt_instruction).await
            }
        }
    }

    fn process_response(
        &self,
        response: &Bytes,
    ) -> (String, bool, Option<usize>) {
        match self {
            ModelServer::Llama(llama) => llama.process_response(response),
            ModelServer::Ollama(ollama) => ollama.process_response(response),
        }
    }

    async fn tokenizer(
        &self,
        content: &str,
    ) -> Result<Option<TokenResponse>, Box<dyn Error>> {
        match self {
            ModelServer::Llama(llama) => llama.tokenizer(content).await,
            ModelServer::Ollama(ollama) => ollama.tokenizer(content).await,
        }
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        model: &Box<dyn PromptModelTrait>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            ModelServer::Llama(llama) => {
                llama
                    .completion(
                        exchanges,
                        model,
                        prompt_instruction,
                        tx,
                        cancel_rx,
                    )
                    .await
            }
            ModelServer::Ollama(ollama) => {
                ollama
                    .completion(
                        exchanges,
                        model,
                        prompt_instruction,
                        tx,
                        cancel_rx,
                    )
                    .await
            }
        }
    }
}

#[async_trait]
pub trait ServerTrait: Send + Sync {
    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        model: &Box<dyn PromptModelTrait>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>>;

    fn process_response(
        &self,
        response: &Bytes,
    ) -> (String, bool, Option<usize>);

    // optional methods
    async fn initialize(
        &mut self,
        _model: &Box<dyn PromptModelTrait>,
        _prompt_instruction: &mut PromptInstruction,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn tokenizer(
        &self,
        _content: &str,
    ) -> Result<Option<TokenResponse>, Box<dyn Error>> {
        Ok(None)
    }

    async fn get_context_size(
        &self,
        _prompt_instruction: &mut PromptInstruction,
    ) -> Result<usize, Box<dyn Error>> {
        Ok(DEFAULT_CONTEXT_SIZE)
    }

    fn get_role_name(&self, prompt_role: PromptRole) -> &'static str {
        match prompt_role {
            PromptRole::User => "user",
            PromptRole::Assistant => "assistant",
            PromptRole::System => "system",
        }
    }
}
