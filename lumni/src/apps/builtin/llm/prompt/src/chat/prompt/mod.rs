mod assistant;
mod builder;
mod role;

pub use assistant::AssistantManager;
pub use builder::PromptInstructionBuilder;
pub use role::PromptRole;
use serde::{Deserialize, Serialize};

pub use super::completion_options::{AssistantOptions, ChatCompletionOptions};
pub use super::conversation::{NewConversation, PromptInstruction};
pub use super::db::{
    ConversationDatabase, ConversationId, Message, MessageId, UserProfile,
    UserProfileDbHandler,
};
pub use super::PERSONAS;

#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    name: String,
    system_prompt: Option<String>,
    prompt_template: Option<String>,
    exchanges: Option<Vec<ChatExchange>>,
}

impl Prompt {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn exchanges(&self) -> Option<&Vec<ChatExchange>> {
        self.exchanges.as_ref()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatExchange {
    pub question: String,
    pub answer: String,
    pub token_length: Option<usize>,
}
