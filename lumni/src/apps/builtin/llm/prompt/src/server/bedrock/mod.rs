mod eventstream;
mod request;

use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use eventstream::EventStreamMessage;
use request::*;
use lumni::{
    AWSCredentials, AWSRequestBuilder, HttpClient, HttpClientError,
    HttpClientErrorHandler, HttpClientResponse,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_post, ChatExchange, ChatHistory, ChatMessage, Endpoints,
    LLMDefinition, PromptInstruction, ServerTrait,
};
pub use crate::external as lumni;

struct AWSErrorHandler;

impl HttpClientErrorHandler for AWSErrorHandler {
    fn handle_error(
        &self,
        response: HttpClientResponse,
        canonical_reason: String,
    ) -> HttpClientError {
        if response.status_code() == 403 {
            if let Some(value) = response.headers().get("x-amzn-errortype") {
                if let Ok(err_type) = value.to_str() {
                    if err_type.starts_with("ExpiredTokenException") {
                        return HttpClientError::HttpError(
                            403,
                            "ExpiredToken".to_string(),
                        );
                    }
                }
            }
        }
        // Fallback if no special handling is needed
        HttpClientError::HttpError(response.status_code(), canonical_reason)
    }
}

pub struct Bedrock {
    http_client: HttpClient,
    endpoints: Endpoints,
    model: Option<LLMDefinition>,
}

impl Bedrock {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // TODO: get region from AWSCredentials
        let bedrock_endpoint =
            "https://bedrock-runtime.us-east-1.amazonaws.com";
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(bedrock_endpoint)?)
            .set_list_models(Url::parse(bedrock_endpoint)?);

        Ok(Bedrock {
            http_client: HttpClient::new()
                .with_error_handler(Arc::new(AWSErrorHandler)),
            endpoints,
            model: None,
        })
    }

    fn completion_api_payload(
        &self,
        _model: &LLMDefinition,
        exchanges: &Vec<ChatExchange>,
        system_prompt: Option<&str>,
    ) -> Result<String, serde_json::Error> {
        // Convert ChatExchange to list of ChatMessages
        let chat_messages: Vec<ChatMessage> =
            ChatHistory::exchanges_to_messages(
                exchanges,
                None, // dont add system prompt for Bedrock, this is added in the system field
                &|role| self.get_role_name(role),
            );

        // Convert ChatMessages to Messages for BedrockRequestPayload
        let messages: Vec<Message> = chat_messages
            .iter()
            .map(|chat_message| Message {
                role: chat_message.role.clone(),
                content: vec![Content {
                    text: Some(chat_message.content.clone()),
                    image: None,
                    document: None,
                    tool_use: None,
                    tool_result: None,
                    guard_content: None,
                }],
            })
            .collect();

        // Cconvert system_prompt to a system message for BedrockRequestPayload
        let system = if let Some(prompt) = system_prompt {
            Some(vec![SystemMessage {
                text: prompt.to_string(),
                guard_content: None,
            }])
        } else {
            None
        };

        let payload = BedrockRequestPayload {
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            guardrail_config: None,
            inference_config: InferenceConfig {
                max_tokens: 1024,
                stop_sequences: None,
                temperature: 0.7,
                top_p: 0.9,
            },
            messages,
            system,
            tool_config: None,
        };
        serde_json::to_string(&payload)
    }
}

#[async_trait]
impl ServerTrait for Bedrock {
    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        _prompt_instruction: &PromptInstruction,
    ) -> Result<(), Box<dyn Error>> {
        self.model = Some(model);
        Ok(())
    }

    fn get_model(&self) -> Option<&LLMDefinition> {
        self.model.as_ref()
    }

    fn process_response(
        &self,
        response_bytes: Bytes,
    ) -> (String, bool, Option<usize>) {
        match EventStreamMessage::from_bytes(response_bytes) {
            Ok(event) => {
                eprintln!("Headers: {:?}", event.headers);
                let event_type = event
                    .headers
                    .get(":event-type")
                    .cloned()
                    .unwrap_or_default();
                eprintln!("Processing {} event", event_type);

                // Parse the payload as JSON
                if let Ok(json) =
                    serde_json::from_slice::<Value>(&event.payload)
                {
                    eprintln!("Successfully parsed JSON: {:?}", json);
                    // TODO: Implement proper processing of the JSON response
                    (format!("{:?}", json), false, None)
                } else {
                    eprintln!("Failed, raw payload: {:?}", event.payload);
                    ("".to_string(), true, None)
                }
            }
            Err(e) => {
                eprintln!("Failed to parse EventStream message: {:?}", e);
                ("".to_string(), true, None)
            }
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

        let resource = HttpClient::percent_encode_with_exclusion(
            &format!("/model/{}/converse-stream", model.get_name()),
            Some(&[b'/', b'.', b'-']),
        );
        let completion_endpoint = self.endpoints.get_completion_endpoint()?;
        let full_url = format!("{}{}", completion_endpoint, resource);

        let data_payload =
            self.completion_api_payload(model, exchanges, Some(system_prompt))?;

        eprintln!("Payload: {:?}", data_payload);
        let payload_hash = Sha256::digest(data_payload.as_bytes())
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();
        let request_builder = AWSRequestBuilder::new(completion_endpoint);
        let credentials = AWSCredentials::from_env()
            .ok()
            .expect("No credentials found");
        let headers = request_builder.generate_headers(
            "POST",
            "bedrock",
            &credentials,
            // resource must be double percent encoded, generate_headers() will percent encode
            // it again. I.e. double percent-encoded, required to get correct v4 sig
            Some(&resource),
            None, // query string
            Some(&payload_hash),
        )?;

        http_post(
            full_url,
            self.http_client.clone(),
            tx,
            data_payload,
            Some(headers),
            cancel_rx,
        )
        .await;
        Ok(())
    }

    async fn list_models(
        &self,
    ) -> Result<Option<Vec<LLMDefinition>>, Box<dyn Error>> {
        let model = LLMDefinition::new(
            "anthropic.claude-3-5-sonnet-20240620-v1:0".to_string(),
        );
        Ok(Some(vec![model]))
    }
}

