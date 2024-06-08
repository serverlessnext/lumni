mod completion;

pub use completion::ChatCompletionResponse;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TokenResponse {
    tokens: Vec<usize>,
}

impl TokenResponse {
    pub fn get_tokens(&self) -> &Vec<usize> {
        &self.tokens
    }
}
