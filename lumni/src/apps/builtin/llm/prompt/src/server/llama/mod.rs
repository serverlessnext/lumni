use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use lumni::api::error::ApplicationError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_get_with_response, http_post, ChatCompletionOptions, ChatMessage,
    Endpoints, HttpClient, LLMDefinition, PromptInstruction, PromptRole,
    ServerSpecTrait, ServerTrait, TokenResponse, DEFAULT_CONTEXT_SIZE,
};
use crate::external as lumni;

pub const DEFAULT_TOKENIZER_ENDPOINT: &str = "http://localhost:8080/tokenize";
pub const DEFAULT_COMPLETION_ENDPOINT: &str =
    "http://localhost:8080/completion";
pub const DEFAULT_SETTINGS_ENDPOINT: &str = "http://localhost:8080/props";

define_and_impl_server_spec!(LlamaSpec);

pub struct Llama {
    spec: LlamaSpec,
    http_client: HttpClient,
    endpoints: Endpoints,
    model: Option<LLMDefinition>,
}

impl Llama {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(DEFAULT_COMPLETION_ENDPOINT)?)
            .set_tokenizer(Url::parse(DEFAULT_TOKENIZER_ENDPOINT)?)
            .set_settings(Url::parse(DEFAULT_SETTINGS_ENDPOINT)?);

        Ok(Llama {
            spec: LlamaSpec {
                name: "Llama".to_string(),
            },
            http_client: HttpClient::new(),
            endpoints,
            model: None,
        })
    }

    fn system_prompt_payload(
        &self,
        prompt_instruction: &PromptInstruction,
    ) -> Option<String> {
        let instruction = prompt_instruction.get_instruction();

        let system_prompt = LlamaServerSystemPrompt::new(
            instruction.to_string(),
            prompt_instruction
                .get_prompt_options()
                .get_role_prefix(PromptRole::User)
                .to_string(),
            prompt_instruction
                .get_prompt_options()
                .get_role_prefix(PromptRole::Assistant)
                .to_string(),
        );
        let payload = LlamaServerPayload {
            prompt: "",
            system_prompt: Some(&system_prompt),
            options: &prompt_instruction.get_completion_options(),
        };
        payload.serialize()
    }

    fn completion_api_payload(
        &self,
        prompt: String,
        prompt_instruction: &PromptInstruction,
    ) -> Result<String, serde_json::Error> {
        let payload = LlamaServerPayload {
            prompt: &prompt,
            system_prompt: None,
            options: prompt_instruction.get_completion_options(),
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
    ) -> (Option<String>, bool, Option<usize>) {
        match LlamaCompletionResponse::extract_content(response) {
            Ok(chat) => (Some(chat.content), chat.stop, chat.tokens_predicted),
            Err(e) => {
                (Some(format!("Failed to parse JSON: {}", e)), true, None)
            }
        }
    }

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        let model = self.get_selected_model()?;
        let formatter = model.get_formatter();
        let prompt = messages
            .into_iter()
            .map(|m| formatter.fmt_prompt_message(&m.role, &m.content))
            .collect::<Vec<String>>()
            .join("\n");

        let data_payload =
            self.completion_api_payload(prompt, prompt_instruction);

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
        let settings = self.get_props().await?;
        let model_file = settings.default_generation_settings.model;
        let model_name = model_file.split('/').last().unwrap().to_lowercase();
        let llm_def = LLMDefinition::new(model_name.to_string());
        Ok(vec![llm_def])
    }

    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        prompt_instruction: &PromptInstruction,
    ) -> Result<(), ApplicationError> {
        self.model = Some(model);

        // Send the system prompt to the completion endpoint at initialization
        let system_prompt_payload =
            self.system_prompt_payload(prompt_instruction);
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

    fn get_model(&self) -> Option<&LLMDefinition> {
        self.model.as_ref()
    }

    async fn get_context_size(
        &self,
        prompt_instruction: &mut PromptInstruction,
    ) -> Result<usize, ApplicationError> {
        let context_size =
            prompt_instruction.get_prompt_options().get_context_size();
        match context_size {
            Some(size) => Ok(size), // Return the context size if it's already set
            None => {
                // fetch the context size, and store it in the prompt options
                let context_size = match self.get_props().await {
                    Ok(props) => props.get_n_ctx(),
                    Err(_) => DEFAULT_CONTEXT_SIZE,
                };
                prompt_instruction
                    .get_prompt_options_mut()
                    .set_context_size(context_size);
                Ok(context_size)
            }
        }
    }

    async fn tokenizer(
        &self,
        content: &str,
    ) -> Result<Option<TokenResponse>, ApplicationError> {
        if let Some(endpoint) = self.endpoints.get_tokenizer() {
            let body_content = serde_json::to_string(
                &json!({ "content": content }),
            )
            .map_err(|e| {
                ApplicationError::ServerConfigurationError(e.to_string())
            })?;
            let body = Bytes::copy_from_slice(body_content.as_bytes());

            let url = endpoint.to_string();
            let mut headers = HashMap::new();
            headers.insert(
                "Content-Type".to_string(),
                "application/json".to_string(),
            );

            let http_response = &self
                .http_client
                .post(&url, Some(&headers), None, Some(&body), None, None)
                .await
                .map_err(|e| ApplicationError::HttpClientError(e))?;

            let response =
                http_response.json::<TokenResponse>().map_err(|e| {
                    ApplicationError::ServerConfigurationError(e.to_string())
                })?;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize)]
struct LlamaServerPayload<'a> {
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<&'a LlamaServerSystemPrompt>,
    #[serde(flatten)]
    options: &'a ChatCompletionOptions,
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
    tokens_predicted: Option<usize>,
}

impl LlamaCompletionResponse {
    pub fn extract_content(
        bytes: Bytes,
    ) -> Result<LlamaCompletionResponse, Box<dyn Error>> {
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
