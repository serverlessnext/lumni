use serde::{Serialize, Deserialize};
use serde_json::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatCompletionResponse {
    content: String,
}

impl ChatCompletionResponse {

    pub fn content(&self) -> String {
        self.content.clone()
    }

    // Method to deserialize JSON and extract the 'content' field
    pub fn extract_content(json_text: &str) -> Result<String, Error> {
        let response: Result<ChatCompletionResponse, _> = serde_json::from_str(json_text);
        match response {
            Ok(parsed) => Ok(parsed.content),
            Err(e) =>  Ok(format!("Failed to parse JSON: {}", e)),
        }
    }
}

