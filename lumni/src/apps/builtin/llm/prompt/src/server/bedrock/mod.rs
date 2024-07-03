mod error;
mod eventstream;
mod request;

use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use error::AWSErrorHandler;
use eventstream::EventStreamMessage;
use lumni::api::error::ApplicationError;
use lumni::{AWSCredentials, AWSRequestBuilder, HttpClient};
use request::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_post, ChatExchange, ChatHistory, ChatMessage, Endpoints,
    LLMDefinition, PromptInstruction, ServerTrait, ServerSpecTrait,
};
pub use crate::external as lumni;

define_and_impl_server_spec!(BedrockSpec);

pub struct Bedrock {
    spec: BedrockSpec,
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
            spec: BedrockSpec {
                name: "Bedrock".to_string(),
            },
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
    fn get_spec(&self) -> &dyn ServerSpecTrait {
        &self.spec
    }

    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        _prompt_instruction: &PromptInstruction,
    ) -> Result<(), ApplicationError> {
        self.model = Some(model);
        Ok(())
    }

    fn get_model(&self) -> Option<&LLMDefinition> {
        self.model.as_ref()
    }

    fn process_response(
        &mut self,
        response_bytes: Bytes,
        _start_of_stream: bool,
    ) -> (Option<String>, bool, Option<usize>) {
        match EventStreamMessage::from_bytes(response_bytes) {
            Ok(event) => {
                let event_type = event
                    .headers
                    .get(":event-type")
                    .cloned()
                    .unwrap_or_default();
                process_event_payload(event_type, event.payload)
            }
            Err(e) => {
                log::error!("Failed to parse EventStreamMessage: {}", e);
                (None, true, None)
            }
        }
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        let model = self.get_selected_model()?;
        let system_prompt = prompt_instruction.get_instruction();

        let resource = HttpClient::percent_encode_with_exclusion(
            &format!("/model/{}/converse-stream", model.get_name()),
            Some(&[b'/', b'.', b'-']),
        );
        let completion_endpoint = self.endpoints.get_completion_endpoint()?;
        let full_url = format!("{}{}", completion_endpoint, resource);

        let data_payload = self
            .completion_api_payload(model, exchanges, Some(system_prompt))
            .map_err(|e| {
                ApplicationError::InvalidUserConfiguration(e.to_string())
            })?;

        let payload_hash = Sha256::digest(data_payload.as_bytes())
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();

        let request_builder = AWSRequestBuilder::new(completion_endpoint);
        let credentials = AWSCredentials::from_env()?;

        let headers = request_builder
            .generate_headers(
                "POST",
                "bedrock",
                &credentials,
                // resource must be double percent encoded, generate_headers() will percent encode
                // it again. I.e. double percent-encoded, required to get correct v4 sig
                Some(&resource),
                None, // query string
                Some(&payload_hash),
            )
            .map_err(|e| {
                ApplicationError::InvalidUserConfiguration(e.to_string())
            })?;

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
    ) -> Result<Vec<LLMDefinition>, ApplicationError> {
        let model = LLMDefinition::new(
            "anthropic.claude-3-5-sonnet-20240620-v1:0".to_string(),
        );
        Ok(vec![model])
    }
}

fn process_event_payload(
    event_type: String,
    payload: Option<Bytes>,
) -> (Option<String>, bool, Option<usize>) {
    let mut stop = false;

    log::debug!("EventType: {:?}", event_type);
    match event_type.as_str() {
        "messageStart" | "contentBlockStart" => {}
        "contentBlockStop" | "messageStop" => stop = true,
        "metadata" => {
            if let Some(json) = parse_payload(payload) {
                if let Some(usage) = json["usage"].as_object() {
                    log::debug!("Usage: {:?}", usage);
                }
                if let Some(metrics) = json["metrics"].as_object() {
                    log::debug!("Metrics: {:?}", metrics);
                }
            }
        }
        "contentBlockDelta" => {
            if let Some(json) = parse_payload(payload) {
                if let Some(text) = json["delta"]["text"].as_str() {
                    log::debug!("Text received: {:?}", text);
                    return (Some(text.to_string()), false, None);
                }
            }
        }
        _ => {
            log::warn!("Unhandled event type: {}", event_type);
        }
    }
    (None, stop, None)
}

fn parse_payload(payload: Option<Bytes>) -> Option<Value> {
    payload.and_then(|p| match serde_json::from_slice(&p) {
        Ok(json) => Some(json),
        Err(_) => {
            log::error!("Failed to parse payload");
            None
        }
    })
}
