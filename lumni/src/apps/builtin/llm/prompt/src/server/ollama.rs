use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_post, http_post_with_response, ChatExchange, ChatHistory, ChatMessage,
    Endpoints, HttpClient, PromptInstruction, PromptModelTrait, ServerTrait,
};

pub const DEFAULT_COMPLETION_ENDPOINT: &str = "http://localhost:11434/api/chat";
pub const DEFAULT_SHOW_ENDPOINT: &str = "http://localhost:11434/api/show";

pub struct Ollama {
    http_client: HttpClient,
    endpoints: Endpoints,
}

impl Ollama {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(DEFAULT_COMPLETION_ENDPOINT)?);

        Ok(Ollama {
            http_client: HttpClient::new(),
            endpoints,
        })
    }

    fn completion_api_payload(
        &self,
        model: &Box<dyn PromptModelTrait>,
        exchanges: &Vec<ChatExchange>,
        system_prompt: Option<&str>,
    ) -> Result<String, serde_json::Error> {
        let messages = ChatHistory::exchanges_to_messages(
            exchanges,
            system_prompt,
            &|role| self.get_role_name(role),
        );

        let payload = ServerPayload {
            model: model.get_model_data().get_name(),
            messages: &messages,
            //options: &self.completion_options,
        };
        serde_json::to_string(&payload)
    }
}

#[async_trait]
impl ServerTrait for Ollama {
    async fn initialize(
        &mut self,
        model: &Box<dyn PromptModelTrait>,
        _prompt_instruction: &mut PromptInstruction,
    ) -> Result<(), Box<dyn Error>> {
        let model_name = model.get_model_data().get_name();
        let payload = OllamaShowPayload {
            name: model.get_model_data().get_name(),
        }
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

    fn process_response(
        &self,
        response: &Bytes,
    ) -> (String, bool, Option<usize>) {
        match OllamaCompletionResponse::extract_content(response) {
            Ok(chat) => (chat.message.content, chat.done, chat.eval_count),
            Err(e) => (format!("Failed to parse JSON: {}", e), true, None),
        }
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        model: &Box<dyn PromptModelTrait>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>> {
        let system_prompt = prompt_instruction.get_instruction();
        let data_payload =
            self.completion_api_payload(model, exchanges, Some(system_prompt));
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
        bytes: &Bytes,
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
