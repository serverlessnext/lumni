use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use lumni::api::error::ApplicationError;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_get_with_response, http_post, http_post_with_response, ChatMessage,
    Endpoints, HttpClient, LLMDefinition, PromptInstruction, ServerSpecTrait,
    ServerTrait,
};
use crate::external as lumni;

pub const DEFAULT_COMPLETION_ENDPOINT: &str = "http://localhost:11434/api/chat";
pub const DEFAULT_SHOW_ENDPOINT: &str = "http://localhost:11434/api/show";
pub const DEFAULT_LIST_MODELS_ENDPOINT: &str =
    "http://localhost:11434/api/tags";

define_and_impl_server_spec!(OllamaSpec);

pub struct Ollama {
    spec: OllamaSpec,
    http_client: HttpClient,
    endpoints: Endpoints,
    model: Option<LLMDefinition>,
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
            model: None,
        })
    }

    fn completion_api_payload(
        &self,
        model: &LLMDefinition,
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
            model: model.get_name(),
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

    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        _prompt_instruction: &PromptInstruction,
    ) -> Result<(), ApplicationError> {
        let payload = OllamaShowPayload {
            name: model.get_name(),
        }
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
                    model.get_name()
                );
                return Err(ApplicationError::ServerConfigurationError(
                    error_message,
                ));
            }
        }
        self.model = Some(model);
        Ok(())
    }

    fn get_model(&self) -> Option<&LLMDefinition> {
        self.model.as_ref()
    }

    fn process_response(
        &mut self,
        response: Bytes,
        _start_of_stream: bool,
    ) -> (Option<String>, bool, Option<usize>) {
        match OllamaCompletionResponse::extract_content(response) {
            Ok(chat) => {
                (Some(chat.message.content), chat.done, chat.eval_count)
            }
            Err(e) => {
                (Some(format!("Failed to parse JSON: {}", e)), true, None)
            }
        }
    }

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        _prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        let model = self.get_selected_model()?;
        let data_payload =
            self.completion_api_payload(model, messages);
        let completion_endpoint = self.endpoints.get_completion_endpoint()?;

        if let Ok(payload) = data_payload {
            http_post(
                completion_endpoint,
                self.http_client.clone(),
                tx,
                payload,
                None,
                cancel_rx,
            )
            .await;
        }
        Ok(())
    }

    async fn list_models(
        &self,
    ) -> Result<Vec<LLMDefinition>, ApplicationError> {
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
        let models = api_response
            .models
            .into_iter()
            .map(|model| {
                let mut llm_def = LLMDefinition::new(model.name);
                llm_def
                    .set_size(model.size)
                    .set_family(model.details.family)
                    .set_description(format!(
                        "Parameter Size: {}",
                        model.details.parameter_size
                    ));

                llm_def
            })
            .collect();
        Ok(models)
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
    eval_count: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
struct OllamaResponseMessage {
    role: String,
    content: String,
}

impl OllamaCompletionResponse {
    pub fn extract_content(
        bytes: Bytes,
    ) -> Result<OllamaCompletionResponse, Box<dyn Error>> {
        let text = String::from_utf8(bytes.to_vec())?;

        // remove 'data: ' prefix if present
        let json_text = if let Some(json_text) = text.strip_prefix("data: ") {
            json_text
        } else {
            &text
        };
        Ok(serde_json::from_str(json_text)?) // Deserialize the JSON text
    }
}
