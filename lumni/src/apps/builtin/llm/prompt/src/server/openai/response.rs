use std::collections::HashMap;

use bytes::Bytes;
use serde::Deserialize;
use serde_json::{Result as JsonResult, Value};

use super::{CompletionResponse, CompletionStats};

#[derive(Debug, Deserialize)]
pub struct OpenAIResponsePayload {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

pub struct StreamParser {
    buffer: String,
    stop_received: bool,
    usage: Option<Usage>,
}

impl StreamParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            stop_received: false,
            usage: None,
        }
    }

    pub fn process_chunk(
        &mut self,
        chunk: Bytes,
        start_of_stream: bool,
    ) -> Option<CompletionResponse> {
        if start_of_stream {
            self.buffer.clear();
            self.stop_received = false;
            self.usage = None;
        }

        let chunk_str = String::from_utf8_lossy(&chunk);
        self.buffer.push_str(&chunk_str);
        let mut final_content = String::new();

        while let Some(end) = self.buffer.find("\n\n") {
            let message = &self.buffer[..end];
            if message.starts_with("data: ") {
                let json_str = &message[6..];
                log::debug!("Received: {}", json_str);
                match self.parse_payload(json_str) {
                    Ok((content, is_stop, usage)) => {
                        final_content.push_str(&content);
                        if is_stop {
                            self.stop_received = true;
                        }
                        if let Some(usage) = usage {
                            self.usage = Some(usage);
                        }
                    }
                    Err(e) => {
                        if self.stop_received {
                            // openai finishes with [DONE] which is non-valid JSON
                            // when stop-condition is met, we can safely ignore it
                            break;
                        }
                        return Some(CompletionResponse::new_final(
                            format!("Failed to parse JSON: {}", e),
                            None,
                        ));
                    }
                }
            }
            self.buffer = self.buffer[end + 2..].to_string();
        }

        if final_content.is_empty() && !self.stop_received {
            Some(CompletionResponse::new())
        } else if self.stop_received {
            let last_token_received_at = 0; // TODO: Implement this
            Some(CompletionResponse::new_final(
                final_content,
                self.usage.as_ref().map(|usage| CompletionStats {
                    last_token_received_at,
                    tokens_predicted: Some(usage.completion_tokens as usize),
                    tokens_in_prompt: Some(usage.prompt_tokens as usize),
                    ..Default::default()
                }),
            ))
        } else {
            Some(CompletionResponse::new_content(final_content))
        }
    }

    fn parse_payload(
        &self,
        json_str: &str,
    ) -> JsonResult<(String, bool, Option<Usage>)> {
        let payload: OpenAIResponsePayload = serde_json::from_str(json_str)?;

        match payload.choices.first() {
            Some(choice) => {
                let content = choice.delta.content.clone().unwrap_or_default();
                let is_stop = choice.finish_reason.is_some();
                Ok((content, is_stop, None))
            }
            None => {
                if let Some(usage) = payload.usage {
                    log::debug!("Usage received: {:?}", usage);
                    Ok((String::new(), true, Some(usage)))
                } else {
                    Ok((String::new(), false, None))
                }
            }
        }
    }
}
