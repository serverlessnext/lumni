use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_get_with_response, http_post, ChatCompletionOptions, ChatExchange,
    ChatHistory, Endpoints, HttpClient, PromptModelTrait, PromptOptions,
    PromptRole, ServerTrait, TokenResponse, DEFAULT_CONTEXT_SIZE,
};

pub const DEFAULT_TOKENIZER_ENDPOINT: &str = "http://localhost:8080/tokenize";
pub const DEFAULT_COMPLETION_ENDPOINT: &str =
    "http://localhost:8080/completion";
pub const DEFAULT_SETTINGS_ENDPOINT: &str = "http://localhost:8080/props";

pub struct Llama {
    http_client: HttpClient,
    endpoints: Endpoints,
    prompt_options: PromptOptions,
    completion_options: ChatCompletionOptions,
}

impl Llama {
    pub fn new(
        prompt_options: PromptOptions,
        completion_options: ChatCompletionOptions,
    ) -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(DEFAULT_COMPLETION_ENDPOINT)?)
            .set_tokenizer(Url::parse(DEFAULT_TOKENIZER_ENDPOINT)?)
            .set_settings(Url::parse(DEFAULT_SETTINGS_ENDPOINT)?);

        Ok(Llama {
            http_client: HttpClient::new(),
            endpoints,
            prompt_options,
            completion_options,
        })
    }

    fn system_prompt_payload(&self, instruction: &str) -> Option<String> {
        let system_prompt = LlamaServerSystemPrompt::new(
            instruction.to_string(),
            self.prompt_options
                .get_role_prefix(PromptRole::User)
                .to_string(),
            self.prompt_options
                .get_role_prefix(PromptRole::Assistant)
                .to_string(),
        );
        let payload = LlamaServerPayload {
            prompt: "",
            system_prompt: Some(&system_prompt),
            options: &self.completion_options,
        };
        payload.serialize()
    }

    fn completion_api_payload(
        &self,
        model: &Box<dyn PromptModelTrait>,
        exchanges: &Vec<ChatExchange>,
    ) -> Result<String, serde_json::Error> {
        let prompt = ChatHistory::exchanges_to_string(model, exchanges);
        let payload = LlamaServerPayload {
            prompt: &prompt,
            system_prompt: None,
            options: &self.completion_options,
        };
        serde_json::to_string(&payload)
    }
}

#[async_trait]
impl ServerTrait for Llama {
    fn set_n_keep(&mut self, n_keep: usize) {
        self.completion_options.set_n_keep(n_keep);
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        model: &Box<dyn PromptModelTrait>,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>> {
        let data_payload = self.completion_api_payload(model, exchanges);

        let completion_endpoint = self.endpoints.get_completion_endpoint()?;
        if let Ok(payload) = data_payload {
            http_post(
                completion_endpoint,
                self.http_client.clone(),
                tx,
                payload,
                cancel_rx,
            )
            .await;
        }
        Ok(())
    }

    async fn put_system_prompt(
        &self,
        system_prompt: &str,
    ) -> Result<(), Box<dyn Error>> {
        let system_prompt_payload = self.system_prompt_payload(system_prompt);
        if let Some(payload) = system_prompt_payload {
            let completion_endpoint =
                self.endpoints.get_completion_endpoint()?;
            http_post(
                completion_endpoint,
                self.http_client.clone(),
                None,
                payload,
                None,
            )
            .await;
        }
        Ok(())
    }

    async fn get_context_size(&mut self) -> Result<usize, Box<dyn Error>> {
        let context_size = self.prompt_options.get_context_size();
        match context_size {
            Some(size) => Ok(size), // Return the context size if it's already set
            None => {
                // fetch the context size, and store it in the prompt options
                let context_size = match self.endpoints.get_settings() {
                    Some(endpoint) => {
                        match http_get_with_response(
                            endpoint.to_string(),
                            self.http_client.clone(),
                        )
                        .await
                        {
                            Ok(response) => {
                                match serde_json::from_slice::<
                                    LlamaServerSettingsResponse,
                                >(
                                    &response
                                ) {
                                    Ok(response_json) => {
                                        response_json.get_n_ctx()
                                    }
                                    Err(_) => DEFAULT_CONTEXT_SIZE, // Fallback on JSON error
                                }
                            }
                            Err(_) => DEFAULT_CONTEXT_SIZE, // Fallback on HTTP request error
                        }
                    }
                    None => DEFAULT_CONTEXT_SIZE, // Fallback if no endpoint is available
                };
                self.prompt_options.set_context_size(context_size);
                Ok(context_size)
            }
        }
    }

    async fn tokenizer(
        &self,
        content: &str,
    ) -> Result<Option<TokenResponse>, Box<dyn Error>> {
        if let Some(endpoint) = self.endpoints.get_tokenizer() {
            let body_content =
                serde_json::to_string(&json!({ "content": content }))?;
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
                .map_err(|e| format!("Error calling tokenizer: {}", e))?;

            let response = http_response.json::<TokenResponse>()?;
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
