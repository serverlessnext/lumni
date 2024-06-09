use std::error::Error;

mod exchange;
mod history;
mod prompt;
mod send;
mod session;

use serde::Deserialize;

pub use exchange::ChatExchange;
pub use history::{ChatHistory, ChatMessage};
use prompt::Prompt;
pub use prompt::PromptInstruction;
pub use send::{http_get_with_response, http_post_with_response, http_post};
pub use session::ChatSession;

pub use super::model::{PromptModel, PromptModelTrait, PromptRole};
pub use super::server::ServerTrait;

// gets PERSONAS from the generated code
include!(concat!(env!("OUT_DIR"), "/llm/prompt/templates.rs"));

pub fn list_assistants() -> Result<Vec<String>, Box<dyn Error>> {
    let prompts: Vec<Prompt> = serde_yaml::from_str(PERSONAS)?;
    let assistants: Vec<String> =
        prompts.iter().map(|p| p.name().to_owned()).collect();
    Ok(assistants)
}


#[derive(Deserialize)]
pub struct TokenResponse {
    tokens: Vec<usize>,
}

impl TokenResponse {
    pub fn get_tokens(&self) -> &Vec<usize> {
        &self.tokens
    }
}
