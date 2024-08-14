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
use request::{OpenAIChatMessage, OpenAIRequestPayload, StreamOptions};
use response::StreamParser;
use serde_json::{json, Value as JsonValue};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_post, ChatMessage, CompletionResponse, CompletionStats,
    ConversationDbHandler, Endpoints, ModelSpec, ServerSpecTrait, ServerTrait,
};
pub use crate::external as lumni;

const OPENAI_COMPLETION_ENDPOINT: &str =
    "https://api.openai.com/v1/chat/completions";

define_and_impl_server_spec!(OpenAISpec); //, "OpenAI");

pub struct OpenAI {
    spec: OpenAISpec,
    http_client: HttpClient,
    endpoints: Endpoints,
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
            stream_parser: StreamParser::new(),
        })
    }

    fn completion_api_payload(
        &self,
        model: &ModelSpec,
        chat_messages: Vec<ChatMessage>,
    ) -> Result<String, serde_json::Error> {
        let messages: Vec<OpenAIChatMessage> = chat_messages
            .iter()
            .map(|m| OpenAIChatMessage {
                role: self.get_role_name(&m.role).to_string(),
                content: m.content.to_string(),
            })
            .collect();

        let openai_request_payload = OpenAIRequestPayload {
            model: model.get_model_name().to_string(),
            messages,
            stream: true,
            stream_options: Some(StreamOptions {
                include_usage: true,
            }),
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

    fn get_profile_settings(&self) -> JsonValue {
        json!({
            "__MODEL_SERVER": "openai",
            "OPENAI_API_KEY": {
                "content": "",
                "encryption_key": "",
            }
        })
    }

    async fn initialize_with_model(
        &mut self,
        _reader: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }

    fn process_response(
        &mut self,
        response_bytes: Bytes,
        start_of_stream: bool,
    ) -> Option<CompletionResponse> {
        self.stream_parser
            .process_chunk(response_bytes, start_of_stream)
    }

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        model: &ModelSpec,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        let completion_endpoint = self.endpoints.get_completion_endpoint()?;
        let data_payload = self
            .completion_api_payload(model, messages.clone())
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

    async fn list_models(&self) -> Result<Vec<ModelSpec>, ApplicationError> {
        Ok(vec![ModelSpec::new_with_validation(
            "openai::gpt-3.5-turbo",
        )?])
    }
}
