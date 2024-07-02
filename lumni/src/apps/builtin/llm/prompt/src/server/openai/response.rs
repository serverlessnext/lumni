use std::error::Error;
use std::collections::HashMap;
use bytes::Bytes;
use serde::Deserialize;
use serde_json::Value;
use serde_json::Result as JsonResult;

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

impl OpenAIResponsePayload {
    // TODO: does not work yet
    // OpenAI sents back split responses, which we need to concatenate first
    pub fn extract_content(bytes: Bytes) -> Result<OpenAIResponsePayload, Box<dyn Error>> {
        // Convert bytes to string, log the raw input
        let text = match String::from_utf8(bytes.to_vec()) {
            Ok(t) => t,
            Err(e) => {
                //eprintln!("Failed to convert bytes to UTF-8: {:?}", e);
                //eprintln!("Raw bytes: {:?}", bytes);
                return Err(Box::new(e));
            }
        };
        //eprintln!("Raw text: {:?}", text);

        // Remove 'data: ' prefix if present
        let json_text = text.strip_prefix("data: ").unwrap_or(&text);
        //eprintln!("JSON text after stripping prefix: {:?}", json_text);

        let parsed_json: Value = match serde_json::from_str(json_text) {
            Ok(v) => v,
            Err(e) => {
                //eprintln!("Failed to parse as generic JSON: {:?}", e);
                //eprintln!("Problematic JSON text: {:?}", json_text);
                return Err(Box::new(e));
            }
        };
        //eprintln!("Parsed generic JSON: {:#?}", parsed_json);

        match serde_json::from_value(parsed_json) {
            Ok(payload) => Ok(payload),
            Err(e) => {
                //eprintln!("Failed to deserialize into OpenAIResponsePayload: {:?}", e);
                Err(Box::new(e))
            }
        }
    }
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

    pub fn process_chunk(&mut self, chunk: Bytes, start_of_stream: bool) -> (Option<String>, bool, Option<usize>) {
        if start_of_stream {
            self.buffer.clear();
            self.stopped_received = false;
        } else if self.stopped_received {
            return (None, true, None);
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
                            return (None, true, None);
                        }
                        return (Some(format!("Failed to parse JSON: {}", e)), true, None);
                    }
                }
            }
            self.buffer = self.buffer[end + 2..].to_string();
        }

        if final_content.is_empty() && !stop {
            (None, false, None)
        } else {
            (Some(final_content), stop, None)
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
