mod connector;
mod display;
mod helpers;
mod model;
mod reader;
mod store;

pub use helpers::system_time_in_milliseconds;
pub use model::{ModelIdentifier, ModelSpec};
pub use reader::ConversationReader;
use serde::{Deserialize, Serialize};
pub use store::ConversationDatabaseStore;

pub use super::ConversationCache;
use super::PromptRole;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelServerName(pub String);

impl ModelServerName {
    pub fn from_str<T: AsRef<str>>(s: T) -> Self {
        ModelServerName(s.as_ref().to_string())
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachmentId(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub name: String,
    pub info: serde_json::Value,
    pub model_identifier: ModelIdentifier,
    pub parent_conversation_id: Option<ConversationId>,
    pub fork_message_id: Option<MessageId>, // New field
    pub completion_options: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub role: PromptRole,
    pub message_type: String,
    pub content: String,
    pub has_attachments: bool,
    pub token_length: Option<i64>,
    pub previous_message_id: Option<MessageId>,
    pub created_at: i64,
    pub vote: i64,  // New field
    pub include_in_prompt: bool,    // New field
    pub is_hidden: bool,    // New field
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentData {
    Uri(String),
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub attachment_id: AttachmentId,
    pub message_id: MessageId,
    pub conversation_id: ConversationId,
    pub data: AttachmentData, // file_uri or file_data
    pub file_type: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
    pub is_deleted: bool,
}
