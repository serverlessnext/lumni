use std::error::Error;
mod db;
mod instruction;
mod options;
mod prompt;
mod send;
mod session;

pub use db::ConversationDatabase;
pub use instruction::PromptInstruction;
pub use options::{ChatCompletionOptions, PromptOptions};
use prompt::Prompt;
pub use send::{http_get_with_response, http_post, http_post_with_response};
use serde::Deserialize;
pub use session::ChatSession;

pub use super::defaults::*;
pub use super::model::PromptRole;
pub use super::server::{CompletionResponse, LLMDefinition, ServerManager};

// gets PERSONAS from the generated code
include!(concat!(env!("OUT_DIR"), "/llm/prompt/templates.rs"));

// TODO: add ability to change assistant
#[allow(dead_code)]
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

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: PromptRole,
    pub content: String,
}
