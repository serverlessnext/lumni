use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_get_with_response, http_post, http_post_with_response, ChatExchange,
    ChatHistory, ChatMessage, Endpoints, HttpClient, LLMDefinition,
    PromptInstruction, ServerTrait,
};

pub const DEFAULT_COMPLETION_ENDPOINT: &str = "http://localhost:11434/api/chat";
pub const DEFAULT_SHOW_ENDPOINT: &str = "http://localhost:11434/api/show";
pub const DEFAULT_LIST_MODELS_ENDPOINT: &str =
    "http://localhost:11434/api/tags";

pub struct Ollama {
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
            http_client: HttpClient::new(),
            endpoints,
            model: None,
        })
    }

    fn completion_api_payload(
        &self,
        model: &LLMDefinition,
        exchanges: &Vec<ChatExchange>,
        system_prompt: Option<&str>,
    ) -> Result<String, serde_json::Error> {
        let messages = ChatHistory::exchanges_to_messages(
            exchanges,
            system_prompt,
            &|role| self.get_role_name(role),
        );

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
    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        _prompt_instruction: &PromptInstruction,
    ) -> Result<(), Box<dyn Error>> {
        self.model = Some(model);
        let model_name =
            self.model.as_ref().expect("Model not available").get_name();

        let payload = OllamaShowPayload { name: model_name }
            .serialize()
            .expect("Failed to serialize show payload");

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
                    model_name
                );
                return Err(error_message.into());
            }
        }
        Ok(())
    }

    fn get_model(&self) -> Option<&LLMDefinition> {
        self.model.as_ref()
    }

    fn process_response(
        &self,
        response: Bytes,
    ) -> (String, bool, Option<usize>) {
        match OllamaCompletionResponse::extract_content(response) {
            Ok(chat) => (chat.message.content, chat.done, chat.eval_count),
            Err(e) => (format!("Failed to parse JSON: {}", e), true, None),
        }
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>> {
        let system_prompt = prompt_instruction.get_instruction();

        let model = self.model.as_ref().expect("Model not available");

        let data_payload =
            self.completion_api_payload(model, exchanges, Some(system_prompt));
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
    ) -> Result<Option<Vec<LLMDefinition>>, Box<dyn Error>> {
        let list_models_endpoint = self.endpoints.get_list_models_endpoint()?;
        let response = http_get_with_response(
            list_models_endpoint.to_string(),
            self.http_client.clone(),
        )
        .await?;

        let api_response: ListModelsApiResponse =
            serde_json::from_slice(&response)?;
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

                Some(llm_def)
            })
            .collect();
        Ok(models)
    }
}

#[derive(Serialize)]
struct ServerPayload<'a> {
    model: &'a str,
    messages: &'a Vec<ChatMessage>,
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
