mod bedrock;
mod endpoints;
mod llama;
mod llm;
mod ollama;

use async_trait::async_trait;
pub use bedrock::Bedrock;
use bytes::Bytes;
pub use endpoints::Endpoints;
pub use llama::Llama;
pub use llm::LLMDefinition;
pub use lumni::HttpClient;
pub use ollama::Ollama;
use tokio::sync::{mpsc, oneshot};

pub use super::chat::{
    http_get_with_response, http_post, http_post_with_response,
    ChatCompletionOptions, ChatExchange, ChatHistory, ChatMessage,
    PromptInstruction, TokenResponse,
};
pub use super::defaults::*;
pub use super::model::{ModelFormatter, ModelFormatterTrait, PromptRole};
use crate::external as lumni;
use lumni::api::error::ApplicationError;

pub const SUPPORTED_MODEL_ENDPOINTS: [&str; 3] = ["llama", "ollama", "bedrock"];

pub enum ModelServer {
    Llama(Llama),
    Ollama(Ollama),
    Bedrock(Bedrock),
}

impl ModelServer {
    pub fn from_str(s: &str) -> Result<Self, ApplicationError> {
        match s {
            "llama" => Ok(ModelServer::Llama(Llama::new().map_err(|e| {
                ApplicationError::ServerConfigurationError(e.to_string())
            })?)),
            "ollama" => Ok(ModelServer::Ollama(Ollama::new().map_err(|e| {
                ApplicationError::ServerConfigurationError(e.to_string())
            })?)),
            "bedrock" => Ok(ModelServer::Bedrock(Bedrock::new().map_err(|e| {
                ApplicationError::ServerConfigurationError(e.to_string())
            })?)),
            _ => Err(ApplicationError::NotImplemented(format!(
                "Server {} not supported",
                s
            ))),
        }
    }
}

#[async_trait]
impl ServerTrait for ModelServer {
    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        prompt_instruction: &PromptInstruction,
    ) -> Result<(), ApplicationError> {
        match self {
            ModelServer::Llama(llama) => {
                llama.initialize_with_model(model, prompt_instruction).await
            }
            ModelServer::Ollama(ollama) => {
                ollama
                    .initialize_with_model(model, prompt_instruction)
                    .await
            }
            ModelServer::Bedrock(bedrock) => {
                bedrock
                    .initialize_with_model(model, prompt_instruction)
                    .await
            }
        }
    }

    fn process_response(
        &self,
        response: Bytes,
    ) -> (Option<String>, bool, Option<usize>) {
        match self {
            ModelServer::Llama(llama) => llama.process_response(response),
            ModelServer::Ollama(ollama) => ollama.process_response(response),
            ModelServer::Bedrock(bedrock) => bedrock.process_response(response),
        }
    }

    async fn get_context_size(
        &self,
        prompt_instruction: &mut PromptInstruction,
    ) -> Result<usize, ApplicationError> {
        match self {
            ModelServer::Llama(llama) => {
                llama.get_context_size(prompt_instruction).await
            }
            ModelServer::Ollama(ollama) => {
                ollama.get_context_size(prompt_instruction).await
            }
            ModelServer::Bedrock(bedrock) => {
                bedrock.get_context_size(prompt_instruction).await
            }
        }
    }

    async fn tokenizer(
        &self,
        content: &str,
    ) -> Result<Option<TokenResponse>, ApplicationError> {
        match self {
            ModelServer::Llama(llama) => llama.tokenizer(content).await,
            ModelServer::Ollama(ollama) => ollama.tokenizer(content).await,
            ModelServer::Bedrock(bedrock) => bedrock.tokenizer(content).await,
        }
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        match self {
            ModelServer::Llama(llama) => {
                llama
                    .completion(exchanges, prompt_instruction, tx, cancel_rx)
                    .await
            }
            ModelServer::Ollama(ollama) => {
                ollama
                    .completion(exchanges, prompt_instruction, tx, cancel_rx)
                    .await
            }
            ModelServer::Bedrock(bedrock) => {
                bedrock
                    .completion(exchanges, prompt_instruction, tx, cancel_rx)
                    .await
            }
        }
    }

    async fn list_models(
        &self,
    ) -> Result<Vec<LLMDefinition>, ApplicationError> {
        match self {
            ModelServer::Llama(llama) => llama.list_models().await,
            ModelServer::Ollama(ollama) => ollama.list_models().await,
            ModelServer::Bedrock(bedrock) => bedrock.list_models().await,
        }
    }

    fn get_model(&self) -> Option<&LLMDefinition> {
        match self {
            ModelServer::Llama(llama) => llama.get_model(),
            ModelServer::Ollama(ollama) => ollama.get_model(),
            ModelServer::Bedrock(bedrock) => bedrock.get_model(),
        }
    }
}

#[async_trait]
pub trait ServerTrait: Send + Sync {
    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        prompt_instruction: &PromptInstruction,
    ) -> Result<(), ApplicationError>;

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError>;

    async fn list_models(
        &self,
    ) -> Result<Vec<LLMDefinition>, ApplicationError>;

    fn get_model(&self) -> Option<&LLMDefinition>;

    fn process_response(
        &self,
        response: Bytes,
    ) -> (Option<String>, bool, Option<usize>);

    async fn tokenizer(
        &self,
        _content: &str,
    ) -> Result<Option<TokenResponse>, ApplicationError> {
        Ok(None)
    }

    async fn token_length(
        &self,
        _content: &str,
    ) -> Result<Option<usize>, ApplicationError> {
        if let Some(token_response) = self.tokenizer(_content).await? {
            Ok(Some(token_response.get_tokens().len()))
        } else {
            Ok(None)
        }
    }

    async fn get_context_size(
        &self,
        _prompt_instruction: &mut PromptInstruction,
    ) -> Result<usize, ApplicationError> {
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
