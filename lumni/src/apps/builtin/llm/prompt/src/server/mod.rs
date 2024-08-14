// ensure spec is loaded first due to macro usage
#[macro_use]
mod spec;

mod bedrock;
mod endpoints;
mod llama;
mod ollama;
mod openai;
mod response;
mod send;

use async_trait::async_trait;
pub use bedrock::Bedrock;
use bytes::Bytes;
pub use endpoints::Endpoints;
pub use llama::Llama;
use lumni::api::error::ApplicationError;
use lumni::HttpClient;
pub use ollama::Ollama;
pub use openai::OpenAI;
pub use response::{CompletionResponse, CompletionStats};
use send::{http_get_with_response, http_post, http_post_with_response};
use serde_json::Value as JsonValue;
pub use spec::ServerSpecTrait;
use tokio::sync::{mpsc, oneshot};

use super::chat::db::{ConversationDbHandler, ModelServerName, ModelSpec};
use super::chat::{ChatMessage, PromptRole};
use super::defaults::*;
use crate::external as lumni;

pub const SUPPORTED_MODEL_ENDPOINTS: [&str; 4] =
    ["llama", "ollama", "bedrock", "openai"];

pub enum ModelServer {
    Llama(Llama),
    Ollama(Ollama),
    Bedrock(Bedrock),
    OpenAI(OpenAI),
}

impl ModelServer {
    pub fn from_str(s: &str) -> Result<Self, ApplicationError> {
        match s {
            "llama" => Ok(ModelServer::Llama(Llama::new().map_err(|e| {
                ApplicationError::ServerConfigurationError(e.to_string())
            })?)),
            "ollama" => {
                Ok(ModelServer::Ollama(Ollama::new().map_err(|e| {
                    ApplicationError::ServerConfigurationError(e.to_string())
                })?))
            }
            "bedrock" => {
                Ok(ModelServer::Bedrock(Bedrock::new().map_err(|e| {
                    ApplicationError::ServerConfigurationError(e.to_string())
                })?))
            }
            "openai" => {
                Ok(ModelServer::OpenAI(OpenAI::new().map_err(|e| {
                    ApplicationError::ServerConfigurationError(e.to_string())
                })?))
            }
            _ => Err(ApplicationError::NotImplemented(format!(
                "{}. Supported server types: {:?}",
                s, SUPPORTED_MODEL_ENDPOINTS
            ))),
        }
    }
}

impl ServerManager for ModelServer {}

#[async_trait]
impl ServerTrait for ModelServer {
    fn get_spec(&self) -> &dyn ServerSpecTrait {
        match self {
            ModelServer::Llama(llama) => llama.get_spec(),
            ModelServer::Ollama(ollama) => ollama.get_spec(),
            ModelServer::Bedrock(bedrock) => bedrock.get_spec(),
            ModelServer::OpenAI(openai) => openai.get_spec(),
        }
    }

    fn get_profile_settings(&self) -> JsonValue {
        match self {
            ModelServer::Llama(llama) => llama.get_profile_settings(),
            ModelServer::Ollama(ollama) => ollama.get_profile_settings(),
            ModelServer::Bedrock(bedrock) => bedrock.get_profile_settings(),
            ModelServer::OpenAI(openai) => openai.get_profile_settings(),
        }
    }

    async fn initialize_with_model(
        &mut self,
        reader: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        match self {
            ModelServer::Llama(llama) => {
                llama.initialize_with_model(reader).await
            }
            ModelServer::Ollama(ollama) => {
                ollama.initialize_with_model(reader).await
            }
            ModelServer::Bedrock(bedrock) => {
                bedrock.initialize_with_model(reader).await
            }
            ModelServer::OpenAI(openai) => {
                openai.initialize_with_model(reader).await
            }
        }
    }

    fn process_response(
        &mut self,
        response: Bytes,
        start_of_stream: bool,
    ) -> Option<CompletionResponse> {
        match self {
            ModelServer::Llama(llama) => {
                llama.process_response(response, start_of_stream)
            }
            ModelServer::Ollama(ollama) => {
                ollama.process_response(response, start_of_stream)
            }
            ModelServer::Bedrock(bedrock) => {
                bedrock.process_response(response, start_of_stream)
            }
            ModelServer::OpenAI(openai) => {
                openai.process_response(response, start_of_stream)
            }
        }
    }

    async fn get_max_context_size(&self) -> Result<usize, ApplicationError> {
        match self {
            ModelServer::Llama(llama) => llama.get_max_context_size().await,
            ModelServer::Ollama(ollama) => ollama.get_max_context_size().await,
            ModelServer::Bedrock(bedrock) => {
                bedrock.get_max_context_size().await
            }
            ModelServer::OpenAI(openai) => openai.get_max_context_size().await,
        }
    }

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        model: &ModelSpec,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        match self {
            ModelServer::Llama(llama) => {
                llama.completion(messages, model, tx, cancel_rx).await
            }
            ModelServer::Ollama(ollama) => {
                ollama.completion(messages, model, tx, cancel_rx).await
            }
            ModelServer::Bedrock(bedrock) => {
                bedrock.completion(messages, model, tx, cancel_rx).await
            }
            ModelServer::OpenAI(openai) => {
                openai.completion(messages, model, tx, cancel_rx).await
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelSpec>, ApplicationError> {
        match self {
            ModelServer::Llama(llama) => llama.list_models().await,
            ModelServer::Ollama(ollama) => ollama.list_models().await,
            ModelServer::Bedrock(bedrock) => bedrock.list_models().await,
            ModelServer::OpenAI(openai) => openai.list_models().await,
        }
    }
}

#[async_trait]
pub trait ServerTrait: Send + Sync {
    fn get_spec(&self) -> &dyn ServerSpecTrait;
    fn get_profile_settings(&self) -> JsonValue;

    async fn initialize_with_model(
        &mut self,
        reader: &ConversationDbHandler,
    ) -> Result<(), ApplicationError>;

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        model: &ModelSpec,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError>;

    async fn list_models(&self) -> Result<Vec<ModelSpec>, ApplicationError>;

    async fn get_default_model(&self) -> Result<ModelSpec, ApplicationError> {
        match self.list_models().await {
            Ok(models) => {
                if models.is_empty() {
                    Err(ApplicationError::ServerConfigurationError(
                        "No models available".to_string(),
                    ))
                } else {
                    Ok(models[0].to_owned())
                }
            }
            Err(e) => Err(e), // propagate error
        }
    }

    fn process_response(
        &mut self,
        response: Bytes,
        start_of_stream: bool,
    ) -> Option<CompletionResponse>;

    async fn get_max_context_size(&self) -> Result<usize, ApplicationError> {
        Ok(DEFAULT_CONTEXT_SIZE)
    }

    fn get_role_name(&self, prompt_role: &PromptRole) -> &'static str {
        match prompt_role {
            PromptRole::User => "user",
            PromptRole::Assistant => "assistant",
            PromptRole::System => "system",
        }
    }
}

#[async_trait]
pub trait ServerManager: ServerTrait {
    async fn setup_and_initialize(
        &mut self,
        db_handler: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        self.initialize_with_model(db_handler).await
    }

    fn server_name(&self) -> ModelServerName {
        ModelServerName::from_str(self.get_spec().name().to_lowercase())
    }
}
