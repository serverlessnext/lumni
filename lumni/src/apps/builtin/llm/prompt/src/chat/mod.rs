use std::error::Error;
mod conversation;
pub mod db;
mod completion_options;
mod prompt;
mod chat_session;

pub use prompt::AssistantManager;
pub use conversation::{ConversationCache, NewConversation, PromptInstruction};
pub use completion_options::ChatCompletionOptions;
use prompt::Prompt;
pub use prompt::PromptRole;
//pub use send::{http_get_with_response, http_post, http_post_with_response};
pub use chat_session::ChatSession;

pub use super::defaults::*;
pub use super::server::{CompletionResponse, ModelServer, ServerManager};
pub use super::tui::{ColorScheme, TextLine, TextSegment};

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

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: PromptRole,
    pub content: String,
}
