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
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_post, ChatMessage, CompletionResponse, CompletionStats,
    ConversationDbHandler, Endpoints, ModelSpec, PromptRole, ServerSpecTrait,
    ServerTrait,
};
pub use crate::external as lumni;

define_and_impl_server_spec!(BedrockSpec);

pub struct Bedrock {
    spec: BedrockSpec,
    http_client: HttpClient,
    endpoints: Endpoints,
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
        })
    }

    fn completion_api_payload(
        &self,
        _model: &ModelSpec,
        chat_messages: &Vec<ChatMessage>,
    ) -> Result<String, serde_json::Error> {
        // Check if the first message is a system prompt
        let system_prompt = match chat_messages.first() {
            Some(chat_message) => {
                if chat_message.role == PromptRole::System {
                    Some(chat_message.content.clone())
                } else {
                    None
                }
            }
            None => None,
        };
        // skip system prompt if it exists
        let skip = if system_prompt.is_some() { 1 } else { 0 };

        // Convert ChatMessages to Messages for BedrockRequestPayload
        let messages: Vec<Message> = chat_messages
            .iter()
            .skip(skip)
            .map(|chat_message| Message {
                role: self.get_role_name(&chat_message.role).to_string(),
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

        // Convert system_prompt to a system message for BedrockRequestPayload
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

    fn get_profile_settings(&self) -> JsonValue {
        json!({
            "MODEL_SERVER": "bedrock",
            "AWS_PROFILE": null,
            "AWS_REGION": null
        })
    }

    async fn initialize_with_model(
        &mut self,
        _reader: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }

    fn process_response(
        &mut self,
        mut response_bytes: Bytes,
        _start_of_stream: bool,
    ) -> Option<CompletionResponse> {
        let mut has_content = false;
        let mut completion_response = CompletionResponse::new();
        let mut tokens_predicted = None;
        let mut tokens_in_prompt = None;

        while !response_bytes.is_empty() {
            match EventStreamMessage::from_bytes(response_bytes) {
                Ok((event, remaining)) => {
                    let event_type = event
                        .headers
                        .get(":event-type")
                        .cloned()
                        .unwrap_or_default();
                    let (
                        response_content,
                        is_final,
                        tokens_pred,
                        tokens_prompt,
                    ) = process_event_payload(event_type, event.payload);

                    if let Some(content) = response_content {
                        completion_response.append(content);
                        has_content = true;
                    }
                    if is_final {
                        completion_response.set_final();
                    }
                    if let Some(t) = tokens_pred {
                        tokens_predicted = Some(t);
                    }
                    if let Some(t) = tokens_prompt {
                        tokens_in_prompt = Some(t);
                    }

                    response_bytes = remaining.unwrap_or_default();
                }
                Err(e) => {
                    log::error!("Failed to parse EventStreamMessage: {}", e);
                    completion_response.set_final();
                    break;
                }
            }
        }

        if tokens_predicted.is_some() || completion_response.is_final {
            // tokens predicted is given, so can assume final token is received
            let last_token_received_at = 0; // TODO: Implement this
            completion_response.stats = Some(CompletionStats {
                last_token_received_at,
                tokens_predicted,
                tokens_in_prompt,
                ..CompletionStats::default()
            });
            Some(completion_response)
        } else if has_content {
            Some(completion_response)
        } else {
            None
        }
    }

    async fn completion(
        &self,
        messages: &Vec<ChatMessage>,
        model: &ModelSpec,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), ApplicationError> {
        let resource = HttpClient::percent_encode_with_exclusion(
            &format!(
                "/model/{}.{}/converse-stream",
                model.get_model_provider(),
                model.get_model_name()
            ),
            Some(&[b'/', b'.', b'-']),
        );
        let completion_endpoint = self.endpoints.get_completion_endpoint()?;
        let full_url = format!("{}{}", completion_endpoint, resource);

        let data_payload =
            self.completion_api_payload(model, messages).map_err(|e| {
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

    async fn list_models(&self) -> Result<Vec<ModelSpec>, ApplicationError> {
        Ok(vec![ModelSpec::new_with_validation(
            "anthropic::claude-3-5-sonnet-20240620-v1:0",
        )?])
    }
}

fn process_event_payload(
    event_type: String,
    payload: Option<Bytes>,
) -> (Option<String>, bool, Option<usize>, Option<usize>) {
    let mut tokens_predicted = None;
    let mut tokens_in_prompt = None;
    log::debug!("EventType: {:?}", event_type);
    match event_type.as_str() {
        "messageStart" | "contentBlockStart" => {
            return (Some("".to_string()), false, None, None);
        }
        "contentBlockStop" | "messageStop" => {}
        "metadata" => {
            if let Some(json) = parse_payload(payload) {
                if let Some(usage) = json["usage"].as_object() {
                    log::debug!("Usage: {:?}", usage);
                    if let Some(output_tokens) = usage["outputTokens"].as_u64()
                    {
                        tokens_predicted = Some(output_tokens as usize);
                    }
                    if let Some(input_tokens) = usage["inputTokens"].as_u64() {
                        tokens_in_prompt = Some(input_tokens as usize);
                    }
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
                    return (Some(text.to_string()), false, None, None);
                } else {
                    return (Some("".to_string()), false, None, None);
                }
            }
        }
        _ => {
            log::warn!("Unhandled event type: {}", event_type);
        }
    }

    // unhandled event is considered as final
    (None, true, tokens_predicted, tokens_in_prompt)
}

fn parse_payload(payload: Option<Bytes>) -> Option<JsonValue> {
    payload.and_then(|p| match serde_json::from_slice(&p) {
        Ok(json) => Some(json),
        Err(_) => {
            log::error!("Failed to parse payload");
            None
        }
    })
}
