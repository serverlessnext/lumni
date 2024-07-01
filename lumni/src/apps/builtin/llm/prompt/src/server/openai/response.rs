use std::error::Error;
use std::collections::HashMap;
use bytes::Bytes;
use serde::Deserialize;
use serde_json::Value;

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
                eprintln!("Failed to convert bytes to UTF-8: {:?}", e);
                eprintln!("Raw bytes: {:?}", bytes);
                return Err(Box::new(e));
            }
        };
        eprintln!("Raw text: {:?}", text);

        // Remove 'data: ' prefix if present
        let json_text = text.strip_prefix("data: ").unwrap_or(&text);
        eprintln!("JSON text after stripping prefix: {:?}", json_text);

        let parsed_json: Value = match serde_json::from_str(json_text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to parse as generic JSON: {:?}", e);
                eprintln!("Problematic JSON text: {:?}", json_text);
                return Err(Box::new(e));
            }
        };
        eprintln!("Parsed generic JSON: {:#?}", parsed_json);

        match serde_json::from_value(parsed_json) {
            Ok(payload) => Ok(payload),
            Err(e) => {
                eprintln!("Failed to deserialize into OpenAIResponsePayload: {:?}", e);
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