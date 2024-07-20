mod formatters;

use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use lumni::api::error::ApplicationError;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_get_with_response, http_post, ChatMessage, CompletionResponse,
    CompletionStats, ConversationReader, Endpoints, HttpClient, ModelSpec,
    ServerSpecTrait, ServerTrait, DEFAULT_CONTEXT_SIZE,
};
use formatters::{ModelFormatter, ModelFormatterTrait};
pub use super::PromptRole;
use crate::external as lumni;

pub const DEFAULT_COMPLETION_ENDPOINT: &str =
    "http://localhost:8080/completion";
pub const DEFAULT_SETTINGS_ENDPOINT: &str = "http://localhost:8080/props";

define_and_impl_server_spec!(LlamaSpec);

pub struct Llama {
    spec: LlamaSpec,
    http_client: HttpClient,
    endpoints: Endpoints,
    formatter: Option<Box<dyn ModelFormatterTrait>>,
    completion_options: Option<serde_json::Value>,
}

impl Llama {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(DEFAULT_COMPLETION_ENDPOINT)?)
            .set_settings(Url::parse(DEFAULT_SETTINGS_ENDPOINT)?);

        Ok(Llama {
            spec: LlamaSpec {
                name: "Llama".to_string(),
            },
            http_client: HttpClient::new(),
            endpoints,
            formatter: None,
            completion_options: None,
        })
    }

    fn set_completion_options(
        &mut self,
        reader: &ConversationReader,
    ) -> Result<(), ApplicationError> {
        let options = reader
            .get_completion_options()
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
        // TODO: should map generic options to Llama-specific options
        self.completion_options = Some(options);
        Ok(())
    }

    fn system_prompt_payload(&self, instruction: String) -> Option<String> {
        let system_prompt = LlamaServerSystemPrompt::new(
            instruction.to_string(),
            format!("### {}", PromptRole::User.to_string()),
            format!("### {}", PromptRole::Assistant.to_string()),
        );
        let payload = LlamaServerPayload {
            prompt: "",
            system_prompt: Some(&system_prompt),
            options: self.completion_options.clone().unwrap_or_default(),
        };
        payload.serialize()
    }

    fn completion_api_payload(
        &self,
        prompt: String,
    ) -> Result<String, serde_json::Error> {
        let payload = LlamaServerPayload {
            prompt: &prompt,
            system_prompt: None,
            options: self.completion_options.clone().unwrap_or_default(),
        };
        serde_json::to_string(&payload)
    }

    async fn get_props(
        &self,
    ) -> Result<LlamaServerSettingsResponse, ApplicationError> {
        let settings_endpoint = self
            .endpoints
            .get_settings()
            .ok_or_else(|| {
                ApplicationError::ServerConfigurationError(
                    "Settings endpoint not set".to_string(),
                )
            })?
            .to_string();

        let result =
            http_get_with_response(settings_endpoint, self.http_client.clone())
                .await;

        match result {
            Ok(response) => Ok(serde_json::from_slice::<
                LlamaServerSettingsResponse,
            >(&response)
            .map_err(|e| {
                ApplicationError::ServerConfigurationError(e.to_string())
            })?),
            Err(e) => Err(ApplicationError::NotReady(e.to_string())),
        }
    }
}

#[async_trait]
impl ServerTrait for Llama {
    fn get_spec(&self) -> &dyn ServerSpecTrait {
        &self.spec
    }

    fn process_response(
        &mut self,
        response: Bytes,
        _start_of_stream: bool,
    ) -> Option<CompletionResponse> {
        match LlamaCompletionResponse::extract_content(response) {
            Ok(completion_response) => Some(completion_response),
            Err(e) => Some(CompletionResponse::new_final(
                format!("Failed to parse JSON: {}", e),
                None,
            )),
        }
    }

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        _model: &ModelSpec,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        let formatter = self.formatter.as_ref().ok_or_else(|| {
            ApplicationError::NotReady("Formatter not initialized".to_string())
        })?;

        let prompt = messages
            .into_iter()
            .map(|m| formatter.fmt_prompt_message(&m.role, &m.content))
            .collect::<Vec<String>>()
            .join("\n");

        let data_payload = self.completion_api_payload(prompt);
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
    ) -> Result<Vec<ModelSpec>, ApplicationError> {
        let settings = self.get_props().await?;
        let model_file = settings.default_generation_settings.model;
        let file_name = model_file.split('/').last().unwrap();
        let model_name = file_name.split('.').next().unwrap().to_lowercase();

        Ok(vec![
            ModelSpec::new_with_validation(&format!("unknown::{}", model_name))?,
        ])
    }

    async fn initialize_with_model(
        &mut self,
        reader: &ConversationReader,
    ) -> Result<(), ApplicationError> {
        let identifier = reader.get_model_identifier()?;
        let model_name = identifier.get_model_name().to_string();
        // Send the system prompt to the completion endpoint at initialization
        self.set_completion_options(reader)?;
        let system_prompt = reader.get_system_prompt()?.unwrap_or_default();
        let system_prompt_payload = self.system_prompt_payload(system_prompt);

        self.formatter = Some(Box::new(ModelFormatter::from_str(&model_name)));

        if let Some(payload) = system_prompt_payload {
            let completion_endpoint =
                self.endpoints.get_completion_endpoint()?;

            http_post(
                completion_endpoint,
                self.http_client.clone(),
                None,
                payload,
                None,
                None,
            )
            .await;
        }
        Ok(())
    }

    async fn get_max_context_size(&self) -> Result<usize, ApplicationError> {
        let context_size = match self.get_props().await {
            Ok(props) => props.get_n_ctx(),
            Err(_) => DEFAULT_CONTEXT_SIZE,
        };
        Ok(context_size)
    }
}

#[derive(Serialize)]
struct LlamaServerPayload<'a> {
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<&'a LlamaServerSystemPrompt>,
    #[serde(flatten)]
    options: serde_json::Value,
}

impl LlamaServerPayload<'_> {
    fn serialize(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LlamaServerSystemPrompt {
    prompt: String,
    anti_prompt: String,
    assistant_name: String,
}

impl LlamaServerSystemPrompt {
    fn new(
        prompt: String,
        anti_prompt: String,
        assistant_name: String,
    ) -> Self {
        LlamaServerSystemPrompt {
            prompt,
            anti_prompt,
            assistant_name,
        }
    }
}

#[derive(Deserialize)]
struct LlamaServerDefaultGenerationSettings {
    n_ctx: usize,
    model: String,
}

#[derive(Deserialize)]
struct LlamaServerSettingsResponse {
    default_generation_settings: LlamaServerDefaultGenerationSettings,
}

impl LlamaServerSettingsResponse {
    fn get_n_ctx(&self) -> usize {
        self.default_generation_settings.n_ctx
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct LlamaCompletionResponse {
    content: String,
    stop: bool,
    tokens_evaluated: Option<usize>, // tokens used in prompt
    tokens_predicted: Option<usize>, // tokens used in completion
}

impl LlamaCompletionResponse {
    pub fn extract_content(
        bytes: Bytes,
    ) -> Result<CompletionResponse, Box<dyn Error>> {
        let text = String::from_utf8(bytes.to_vec())?;
        let json_text = text.strip_prefix("data: ").unwrap_or(&text);
        let response: LlamaCompletionResponse =
            serde_json::from_str(json_text)?;

        if response.stop {
            let last_token_received_at = 0; // TODO: implement this
            Ok(CompletionResponse::new_final(
                response.content,
                Some(CompletionStats {
                    last_token_received_at,
                    tokens_evaluated: response.tokens_evaluated,
                    tokens_predicted: response.tokens_predicted,
                    ..Default::default()
                }),
            ))
        } else {
            Ok(CompletionResponse::new_content(response.content))
        }
    }
}
