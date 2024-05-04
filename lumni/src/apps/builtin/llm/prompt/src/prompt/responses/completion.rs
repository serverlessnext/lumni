use std::error::Error;

use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatCompletionResponse {
    pub content: String,
    pub stop: bool,
}

impl ChatCompletionResponse {
    pub fn to_json_text(text: &str) -> String {
        let message = ChatCompletionResponse {
            content: text.to_string(),
            stop: false,
        };
        serde_json::to_string(&message).unwrap()
    }

    pub fn extract_content(
        bytes: &Bytes,
    ) -> Result<ChatCompletionResponse, Box<dyn Error>> {
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
        // overwrite content in parsed
        //parsed.content = json_text.to_string();
        //parsed.stop = false;
        Ok(parsed)
    }
}
