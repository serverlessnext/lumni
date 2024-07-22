mod cache;
mod instruction;
mod prepare;
pub use cache::ConversationCache;
use db::{ConversationId, MessageId};
pub use instruction::PromptInstruction;
pub use prepare::NewConversation;

pub use super::db;
use super::{ChatCompletionOptions, ChatMessage, PromptRole};

#[derive(Debug, Clone)]
pub struct ParentConversation {
    pub id: ConversationId,
    pub fork_message_id: MessageId,
}
