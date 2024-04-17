
use bytes::Bytes;
use serde::{Serialize, Deserialize};
//use serde_json::Error;
//use std::str::Utf8Error;
use std::io;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatCompletionResponse {
    content: String,
}

impl ChatCompletionResponse {
    pub fn to_json_text(text: &str) -> String {
        let message = ChatCompletionResponse {
            content: text.to_string(),
        };
        serde_json::to_string(&message).unwrap()
    }

    pub fn extract_content(bytes: &Bytes) -> Result<String, Box<dyn Error>> {
        let text = String::from_utf8(bytes.to_vec())?;
       
        // Check if the string starts with 'data: ' (typical for streaming responses)
        // and strip it if it does
        let json_text = if let Some(json_text) = text.strip_prefix("data: ") {
            json_text
        } else {
            &text
        };

        // extract JSON content
        let parsed: ChatCompletionResponse = serde_json::from_str(json_text)?;
        Ok(parsed.content)
    }
}

