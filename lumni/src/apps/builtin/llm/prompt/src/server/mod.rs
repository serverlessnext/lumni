// ensure spec is loaded first due to macro usage
#[macro_use]
mod spec;

mod bedrock;
mod endpoints;
mod llama;
mod llm;
mod ollama;
mod openai;
mod response;

use async_trait::async_trait;
pub use bedrock::Bedrock;
use bytes::Bytes;
pub use endpoints::Endpoints;
pub use llama::Llama;
pub use llm::LLMDefinition;
use lumni::api::error::ApplicationError;
pub use lumni::HttpClient;
pub use ollama::Ollama;
pub use openai::OpenAI;
pub use response::StreamResponse;
pub use spec::ServerSpecTrait;
use tokio::sync::{mpsc, oneshot};

pub use super::chat::{
    http_get_with_response, http_post, http_post_with_response,
    ChatCompletionOptions, ChatMessage, PromptInstruction, TokenResponse,
};
pub use super::defaults::*;
pub use super::model::{ModelFormatter, ModelFormatterTrait, PromptRole};
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
            ModelServer::OpenAI(openai) => {
                openai
                    .initialize_with_model(model, prompt_instruction)
                    .await
            }
        }
    }

    fn process_response(
        &mut self,
        response: Bytes,
        start_of_stream: bool,
    ) -> Option<StreamResponse> {
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
            ModelServer::OpenAI(openai) => {
                openai.get_context_size(prompt_instruction).await
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
            ModelServer::OpenAI(openai) => openai.tokenizer(content).await,
        }
    }

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        match self {
            ModelServer::Llama(llama) => {
                llama
                    .completion(messages, prompt_instruction, tx, cancel_rx)
                    .await
            }
            ModelServer::Ollama(ollama) => {
                ollama
                    .completion(messages, prompt_instruction, tx, cancel_rx)
                    .await
            }
            ModelServer::Bedrock(bedrock) => {
                bedrock
                    .completion(messages, prompt_instruction, tx, cancel_rx)
                    .await
            }
            ModelServer::OpenAI(openai) => {
                openai
                    .completion(messages, prompt_instruction, tx, cancel_rx)
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
            ModelServer::OpenAI(openai) => openai.list_models().await,
        }
    }

    fn get_model(&self) -> Option<&LLMDefinition> {
        match self {
            ModelServer::Llama(llama) => llama.get_model(),
            ModelServer::Ollama(ollama) => ollama.get_model(),
            ModelServer::Bedrock(bedrock) => bedrock.get_model(),
            ModelServer::OpenAI(openai) => openai.get_model(),
        }
    }
}

#[async_trait]
pub trait ServerTrait: Send + Sync {
    fn get_spec(&self) -> &dyn ServerSpecTrait;

    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        prompt_instruction: &PromptInstruction,
    ) -> Result<(), ApplicationError>;

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError>;

    async fn list_models(&self)
        -> Result<Vec<LLMDefinition>, ApplicationError>;

    fn get_model(&self) -> Option<&LLMDefinition>;

    fn get_selected_model(&self) -> Result<&LLMDefinition, ApplicationError> {
        match self.get_model() {
            Some(m) => Ok(m),
            None => {
                Err(ApplicationError::NotReady("No model selected".to_string()))
            }
        }
    }

    async fn get_default_model(&self) -> Option<LLMDefinition> {
        match self.list_models().await {
            Ok(models) => {
                if models.is_empty() {
                    log::warn!("Received empty model list");
                    None
                } else {
                    log::debug!("Available models: {:?}", models);
                    Some(models[0].to_owned())
                }
            }
            Err(e) => {
                log::error!("Failed to list models: {}", e);
                None
            }
        }
    }

    fn process_response(
        &mut self,
        response: Bytes,
        start_of_stream: bool,
    ) -> Option<StreamResponse>;

    async fn tokenizer(
        &self,
        _content: &str,
    ) -> Result<Option<TokenResponse>, ApplicationError> {
        Ok(None)
    }

    async fn get_context_size(
        &self,
        _prompt_instruction: &mut PromptInstruction,
    ) -> Result<usize, ApplicationError> {
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
        model: LLMDefinition,
        prompt_instruction: &mut PromptInstruction,
    ) -> Result<(), ApplicationError> {
        log::debug!("Initializing server with model: {:?}", model);
        // update completion options from the model, i.e. stop tokens
        prompt_instruction.set_model(&model);

        // TODO: not fully implement yet
        // requires both model and server to be ready, and then run
        // in correct order
        let instruction = prompt_instruction.get_instruction();
        if instruction.is_empty() {
            prompt_instruction.set_system_token_length(Some(0));
        } else {
            let token_length = if let Some(token_response) =
                self.tokenizer(instruction).await?
            {
                Some(token_response.get_tokens().len())
            } else {
                None
            };
            prompt_instruction.set_system_token_length(token_length);
        };
        self.initialize_with_model(model, prompt_instruction).await
    }

    fn server_name(&self) -> &str {
        self.get_spec().name()
    }
}
