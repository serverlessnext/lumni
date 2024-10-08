use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_get_with_response, http_post, http_post_with_response,
    ApplicationError, ChatEvent, ChatMessage, CompletionResponse,
    CompletionStats, ConversationDbHandler, Endpoints, HttpClient, ModelSpec,
    ServerSpecTrait, ServerTrait,
};

pub const DEFAULT_COMPLETION_ENDPOINT: &str = "http://localhost:11434/api/chat";
pub const DEFAULT_SHOW_ENDPOINT: &str = "http://localhost:11434/api/show";
pub const DEFAULT_LIST_MODELS_ENDPOINT: &str =
    "http://localhost:11434/api/tags";

define_and_impl_server_spec!(OllamaSpec);

pub struct Ollama {
    spec: OllamaSpec,
    http_client: HttpClient,
    endpoints: Endpoints,
}

impl Ollama {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(DEFAULT_COMPLETION_ENDPOINT)?)
            .set_list_models(Url::parse(DEFAULT_LIST_MODELS_ENDPOINT)?);

        Ok(Ollama {
            spec: OllamaSpec {
                name: "Ollama".to_string(),
            },
            http_client: HttpClient::new(),
            endpoints,
        })
    }

    fn completion_api_payload(
        &self,
        model: &ModelSpec,
        chat_messages: &Vec<ChatMessage>,
    ) -> Result<String, serde_json::Error> {
        let messages: Vec<OllamaChatMessage> = chat_messages
            .iter()
            .map(|m| OllamaChatMessage {
                role: self.get_role_name(&m.role).to_string(),
                content: m.content.to_string(),
            })
            .collect();

        let payload = ServerPayload {
            model: model.get_model_name(),
            messages: &messages,
            //options: &self.completion_options,
        };
        serde_json::to_string(&payload)
    }
}

#[async_trait]
impl ServerTrait for Ollama {
    fn get_spec(&self) -> &dyn ServerSpecTrait {
        &self.spec
    }

    fn provider_configuration(&self) -> JsonValue {
        json!({}) // TODO: add endpoint configuration
    }

    async fn initialize_with_model(
        &mut self,
        handler: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        let identifier =
            handler.fetch_model_identifier().await.map_err(|e| {
                ApplicationError::NotReady(format!(
                    "Cannot get model identifier: {}",
                    e.to_string()
                ))
            })?;
        let model_name = identifier.get_model_name().to_string();
        let payload = OllamaShowPayload { name: &model_name }
            .serialize()
            .ok_or_else(|| {
                ApplicationError::ServerConfigurationError(
                    "Failed to serialize show payload".to_string(),
                )
            })?;

        let response = http_post_with_response(
            DEFAULT_SHOW_ENDPOINT.to_string(),
            self.http_client.clone(),
            payload,
        )
        .await;

        if let Ok(response) = response {
            // check if model is available by validating the response format
            // at this moment we not yet need the response itself
            if OllamaShowResponse::extract_content(&response).is_err() {
                let error_message = format!(
                    "Failed to get model information for: {}",
                    model_name,
                );
                return Err(ApplicationError::ServerConfigurationError(
                    error_message,
                ));
            }
        }
        Ok(())
    }

    fn process_response(
        &mut self,
        response: Bytes,
        _start_of_stream: bool,
    ) -> Option<CompletionResponse> {
        match OllamaCompletionResponse::extract_content(response) {
            Ok(completion_response) => Some(completion_response),
            Err(e) => Some(CompletionResponse::new_final(
                format!("Ollama: failed to parse JSON: {}", e),
                None,
            )),
        }
    }

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        model: &ModelSpec,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
        event_sender: Option<mpsc::Sender<ChatEvent>>,
    ) -> Result<(), ApplicationError> {
        let data_payload = self.completion_api_payload(model, messages);
        let completion_endpoint = self.endpoints.get_completion_endpoint()?;

        if let Ok(payload) = data_payload {
            http_post(
                completion_endpoint,
                self.http_client.clone(),
                tx,
                payload,
                None,
                cancel_rx,
                event_sender,
            )
            .await;
        }
        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<ModelSpec>, ApplicationError> {
        let list_models_endpoint = self.endpoints.get_list_models_endpoint()?;
        let response = http_get_with_response(
            list_models_endpoint.to_string(),
            self.http_client.clone(),
        )
        .await
        .map_err(|e| {
            ApplicationError::NotReady(format!(
                "Cannot get model list: {}",
                e.to_string()
            ))
        })?;

        let api_response: ListModelsApiResponse =
            serde_json::from_slice(&response).map_err(|e| {
                ApplicationError::ServerConfigurationError(format!(
                    "Failed to parse list models response: {}",
                    e
                ))
            })?;

        let models: Result<Vec<ModelSpec>, ApplicationError> = api_response
            .models
            .into_iter()
            .map(|model| {
                let model_identifier =
                    format!("{}::{}", "unknown", model.name.to_lowercase());
                let mut model_spec =
                    ModelSpec::new_with_validation(&model_identifier)?;

                model_spec
                    .set_size(model.size)
                    .set_family(&model.details.family)
                    .set_description(&format!(
                        "Parameter Size: {}",
                        model.details.parameter_size
                    ));

                Ok(model_spec)
            })
            .collect();
        models
    }
}

#[derive(Serialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ServerPayload<'a> {
    model: &'a str,
    messages: &'a Vec<OllamaChatMessage>,
    // TODO: reformat and pass options to ollama
    //#[serde(flatten)]
    //    options: &'a ChatCompletionOptions,
}

impl ServerPayload<'_> {
    #[allow(dead_code)]
    // TODO: reformat and pass options to ollama
    fn serialize(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

#[derive(Serialize)]
struct OllamaShowPayload<'a> {
    name: &'a str,
}

impl OllamaShowPayload<'_> {
    fn serialize(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

// used to check if response can deserialize
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ListModelsApiResponse {
    models: Vec<ModelDetail>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ModelDetail {
    name: String,
    modified_at: String,
    size: usize,
    digest: String,
    details: ModelDetails,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ModelDetails {
    format: String,
    family: String,
    families: Option<Vec<String>>,
    parameter_size: String,
    quantization_level: String,
}

// used to check if response can deserialize
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct OllamaShowResponse {
    modelfile: String,
    details: OllamaShowResponseDetails,
}

impl OllamaShowResponse {
    pub fn extract_content(
        bytes: &Bytes,
    ) -> Result<OllamaShowResponse, Box<dyn Error>> {
        let text = String::from_utf8(bytes.to_vec())?;
        Ok(serde_json::from_str(&text)?)
    }
}

// used to check if response can deserialize
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct OllamaShowResponseDetails {
    format: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OllamaCompletionResponse {
    model: String,
    created_at: String,
    message: OllamaResponseMessage,
    done: bool,
    total_duration: Option<u64>, // total duration in nanoseconds
    prompt_eval_count: Option<usize>, // tokens used in prompt
    eval_count: Option<usize>,   // tokens used in completion
}

#[derive(Serialize, Deserialize, Debug)]
struct OllamaResponseMessage {
    role: String,
    content: String,
}

impl OllamaCompletionResponse {
    pub fn extract_content(
        bytes: Bytes,
    ) -> Result<CompletionResponse, Box<dyn Error>> {
        let json_text = String::from_utf8(bytes.to_vec())?;
        let response: OllamaCompletionResponse =
            serde_json::from_str(&json_text)?;

        let content = response.message.content.clone();

        if response.done {
            let last_token_received_at = 0; // TODO: implement this
            Ok(CompletionResponse::new_final(
                content,
                Some(CompletionStats {
                    last_token_received_at,
                    total_duration: response
                        .total_duration
                        .filter(|&d| d > 0)
                        .map(|d| (d / 1_000_000) as usize),
                    tokens_in_prompt: response.prompt_eval_count,
                    tokens_predicted: response.eval_count,
                    ..Default::default()
                }),
            ))
        } else {
            Ok(CompletionResponse::new_content(content))
        }
    }
}
