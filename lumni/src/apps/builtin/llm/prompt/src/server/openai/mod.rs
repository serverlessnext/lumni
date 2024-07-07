mod credentials;
mod error;
mod request;
mod response;

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use credentials::OpenAICredentials;
use error::OpenAIErrorHandler;
use lumni::api::error::ApplicationError;
use lumni::HttpClient;
use request::OpenAIRequestPayload;
use response::StreamParser;
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_post, ChatExchange, ChatHistory, ChatMessage, Endpoints,
    LLMDefinition, PromptInstruction, ServerSpecTrait, ServerTrait,
};
pub use crate::external as lumni;

const OPENAI_COMPLETION_ENDPOINT: &str =
    "https://api.openai.com/v1/chat/completions";

define_and_impl_server_spec!(OpenAISpec); //, "OpenAI");

pub struct OpenAI {
    spec: OpenAISpec,
    http_client: HttpClient,
    endpoints: Endpoints,
    model: Option<LLMDefinition>,
    stream_parser: StreamParser,
}

impl OpenAI {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(OPENAI_COMPLETION_ENDPOINT)?);

        Ok(OpenAI {
            spec: OpenAISpec {
                name: "OpenAI".to_string(),
            },
            http_client: HttpClient::new()
                .with_error_handler(Arc::new(OpenAIErrorHandler)),
            endpoints,
            model: None,
            stream_parser: StreamParser::new(),
        })
    }

    fn completion_api_payload(
        &self,
        model: &LLMDefinition,
        exchanges: &Vec<ChatExchange>,
        system_prompt: Option<&str>,
    ) -> Result<String, serde_json::Error> {
        let messages: Vec<ChatMessage> = ChatHistory::exchanges_to_messages(
            exchanges,
            system_prompt,
            &|role| self.get_role_name(role),
        );

        let openai_request_payload = OpenAIRequestPayload {
            model: model.get_name().to_string(),
            messages,
            stream: true,
            frequency_penalty: None,
            stop: None,
            temperature: Some(0.7),
            top_p: None,
            max_tokens: None,
            presence_penalty: None,
            logprobs: None,
            best_of: None,
        };
        openai_request_payload.to_json()
    }
}

#[async_trait]
impl ServerTrait for OpenAI {
    fn get_spec(&self) -> &dyn ServerSpecTrait {
        &self.spec
    }

    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        _prompt_instruction: &PromptInstruction,
    ) -> Result<(), ApplicationError> {
        self.model = Some(model);
        Ok(())
    }

    fn get_model(&self) -> Option<&LLMDefinition> {
        self.model.as_ref()
    }

    fn process_response(
        &mut self,
        response_bytes: Bytes,
        start_of_stream: bool,
    ) -> (Option<String>, bool, Option<usize>) {
        self.stream_parser
            .process_chunk(response_bytes, start_of_stream)
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        let model = self.get_selected_model()?;
        let system_prompt = prompt_instruction.get_instruction();

        let completion_endpoint = self.endpoints.get_completion_endpoint()?;
        let data_payload = self
            .completion_api_payload(model, exchanges, Some(system_prompt))
            .map_err(|e| {
                ApplicationError::InvalidUserConfiguration(e.to_string())
            })?;

        let credentials = OpenAICredentials::from_env()?;

        let mut headers = HashMap::new();
        headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", credentials.get_api_key()),
        );

        http_post(
            completion_endpoint,
            self.http_client.clone(),
            tx,
            data_payload,
            Some(headers),
            cancel_rx,
        )
        .await;
        Ok(())
    }

    async fn list_models(
        &self,
    ) -> Result<Vec<LLMDefinition>, ApplicationError> {
        let model = LLMDefinition::new("gpt-3.5-turbo".to_string());
        Ok(vec![model])
    }
}