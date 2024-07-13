use std::collections::HashMap;

use bytes::Bytes;
use serde::Deserialize;
use serde_json::{Result as JsonResult, Value};

use crate::apps::builtin::llm::prompt::src::server::StreamResponse;

#[derive(Debug, Deserialize)]
pub struct OpenAIResponsePayload {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(skip)]
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
    pub model_tokens: u32,
}

pub struct StreamParser {
    buffer: String,
    stopped_received: bool,
}

impl StreamParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            stopped_received: false,
        }
    }

    pub fn process_chunk(
        &mut self,
        chunk: Bytes,
        start_of_stream: bool,
    ) -> Option<StreamResponse> {
        if start_of_stream {
            self.buffer.clear();
            self.stopped_received = false;
        } else if self.stopped_received {
            return None;
        }

        let chunk_str = String::from_utf8_lossy(&chunk);
        self.buffer.push_str(&chunk_str);

        let mut final_content = String::new();
        let mut stop = false;

        while let Some(end) = self.buffer.find("\n\n") {
            let message = &self.buffer[..end];
            if message.starts_with("data: ") {
                let json_str = &message[6..];
                log::debug!("Received: {}", json_str);

                match self.parse_payload(json_str) {
                    Ok((content, is_stop)) => {
                        final_content.push_str(&content);
                        if is_stop {
                            stop = true;
                            self.stopped_received = true;
                        }
                    }
                    Err(e) => {
                        if stop {
                            return None;
                        }
                        return Some(StreamResponse::new_with_content(
                            format!("Failed to parse JSON: {}", e),
                            None,
                            true,
                        ));
                    }
                }
            }
            self.buffer = self.buffer[end + 2..].to_string();
        }

        if final_content.is_empty() && !stop {
            Some(StreamResponse::new())
        } else {
            Some(StreamResponse::new_with_content(final_content, None, stop))
        }
    }

    fn parse_payload(&self, json_str: &str) -> JsonResult<(String, bool)> {
        let payload: OpenAIResponsePayload = serde_json::from_str(json_str)?;
        if let Some(choice) = payload.choices.get(0) {
            let content = choice.delta.content.clone().unwrap_or_default();
            let is_stop = choice.finish_reason.is_some();
            Ok((content, is_stop))
        } else {
            Ok((String::new(), false))
        }
    }
}
