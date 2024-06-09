use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_post, ChatCompletionOptions, ChatExchange,
    ChatHistory, ChatMessage, Endpoints, HttpClient, PromptInstruction,
    PromptModelTrait, PromptOptions, ServerTrait,
    DEFAULT_CONTEXT_SIZE,
};

pub const DEFAULT_COMPLETION_ENDPOINT: &str = "http://localhost:11434/api/chat";

pub struct Ollama {
    http_client: HttpClient,
    endpoints: Endpoints,
    instruction: PromptInstruction,
    prompt_options: PromptOptions,
    completion_options: ChatCompletionOptions,
}

impl Ollama {
    pub fn new(
        instruction: PromptInstruction,
        prompt_options: PromptOptions,
        completion_options: ChatCompletionOptions,
    ) -> Result<Self, Box<dyn Error>> {
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(DEFAULT_COMPLETION_ENDPOINT)?);

        Ok(Ollama {
            http_client: HttpClient::new(),
            endpoints,
            instruction,
            prompt_options,
            completion_options,
        })
    }

    fn completion_api_payload(
        &self,
        model: &Box<dyn PromptModelTrait>,
        exchanges: &Vec<ChatExchange>,
    ) -> Result<String, serde_json::Error> {
        let messages = ChatHistory::exchanges_to_messages(
            exchanges,
            Some(self.instruction.get_instruction()),
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
    fn prompt_instruction(&self) -> &PromptInstruction {
        &self.instruction
    }

    fn prompt_instruction_mut(&mut self) -> &mut PromptInstruction {
        &mut self.instruction
    }

    fn process_prompt_response(&self, response: &Bytes) -> (String, bool) {
        match OllamaCompletionResponse::extract_content(response) {
            Ok(chat) => {
                (chat.message.content, chat.done)
            },
            Err(e) => (format!("Failed to parse JSON: {}", e), true),
        }
    }

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

    async fn get_context_size(&mut self) -> Result<usize, Box<dyn Error>> {
        Ok(DEFAULT_CONTEXT_SIZE)
    }
}

#[derive(Serialize)]
struct ServerPayload<'a> {
    model: &'a str,
    messages: &'a Vec<ChatMessage>,
    //#[serde(flatten)]
    //    options: &'a ChatCompletionOptions,
}

impl ServerPayload<'_> {
    fn serialize(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}


#[derive(Serialize, Deserialize, Debug)]
struct OllamaCompletionResponse {
    model: String,
    created_at: String,
    message: OllamaResponseMessage,
    done: bool,
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
        Ok(serde_json::from_str(json_text)?)    // Deserialize the JSON text
    }
}
