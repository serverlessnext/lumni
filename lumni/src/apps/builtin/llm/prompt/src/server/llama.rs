use std::error::Error;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

use super::defaults::*;
use super::options::PromptOptions;
use super::{
    http_get_with_response, http_post, ChatCompletionOptions, Endpoints,
    HttpClient, PromptModelTrait, PromptRole, ServerTrait,
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
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(DEFAULT_COMPLETION_ENDPOINT)?)
            .set_tokenizer(Url::parse(DEFAULT_TOKENIZER_ENDPOINT)?)
            .set_settings(Url::parse(DEFAULT_SETTINGS_ENDPOINT)?);

        Ok(Llama {
            http_client: HttpClient::new(),
            endpoints,
            prompt_options: PromptOptions::new(),
            completion_options: ChatCompletionOptions::new()
                .set_temperature(DEFAULT_TEMPERATURE)
                .set_n_predict(DEFAULT_N_PREDICT)
                .set_cache_prompt(true)
                .set_stream(true),
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
}

#[async_trait]
impl ServerTrait for Llama {
    fn get_prompt_options(&self) -> &PromptOptions {
        &self.prompt_options
    }

    fn get_completion_options(&self) -> &ChatCompletionOptions {
        &self.completion_options
    }

    fn get_endpoints(&self) -> &Endpoints {
        &self.endpoints
    }

    fn update_options_from_json(&mut self, json: &str) {
        self.prompt_options.update_from_json(json);
        self.completion_options.update_from_json(json);
    }

    fn update_options_from_model(&mut self, model: &dyn PromptModelTrait) {
        self.completion_options.update_from_model(model);
    }

    fn set_n_keep(&mut self, n_keep: usize) {
        self.completion_options.set_n_keep(n_keep);
    }

    fn completion_api_payload(
        &self,
        prompt: String,
    ) -> Result<String, serde_json::Error> {
        let payload = LlamaServerPayload {
            prompt: &prompt,
            system_prompt: None,
            options: self.get_completion_options(),
        };
        serde_json::to_string(&payload)
    }

    async fn put_system_prompt(
        &self,
        system_prompt: &str,
    ) -> Result<(), Box<dyn Error>> {
        let system_prompt_payload = self.system_prompt_payload(system_prompt);
        if let Some(payload) = system_prompt_payload {
            let completion_endpoint = self.completion_endpoint()?;
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
        let context_size = self.get_prompt_options()
            .get_context_size();
        match context_size {
            Some(size) => Ok(size), // Return the context size if it's already set
            None => {   // fetch the context size, and store it in the prompt options
                let context_size = match self.get_endpoints().get_settings() {
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
                                >(&response)
                                {
                                    Ok(response_json) => response_json.get_n_ctx(),
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
