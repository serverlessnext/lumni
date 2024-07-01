use std::env;

use lumni::api::error::ApplicationError;
pub use crate::external as lumni;


#[derive(Clone)]
pub struct OpenAICredentials {
    api_key: String,
}

impl OpenAICredentials {
    pub fn from_env() -> Result<OpenAICredentials, ApplicationError> {
        let api_key = env::var("OPENAI_API_KEY").map_err(|_| {
            ApplicationError::InvalidCredentials(
                "OPENAI_API_KEY not found in environment".to_string(),
            )
        })?;
        Ok(OpenAICredentials { api_key })
    }

    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }
}
