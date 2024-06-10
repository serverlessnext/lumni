use std::error::Error;

mod exchange;
mod history;
mod instruction;
mod options;
mod prompt;
mod send;
mod session;

pub use exchange::ChatExchange;
pub use history::{ChatHistory, ChatMessage};
pub use instruction::PromptInstruction;
pub use options::{ChatCompletionOptions, PromptOptions};
use prompt::Prompt;
pub use send::{http_get_with_response, http_post, http_post_with_response};
use serde::Deserialize;
pub use session::ChatSession;

pub use super::defaults::*;
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
