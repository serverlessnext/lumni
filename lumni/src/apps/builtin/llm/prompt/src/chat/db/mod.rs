use std::error::Error;
use std::fmt;

mod connector;
mod conversations;
mod display;
mod encryption;
mod model;
mod store;
mod user_profiles;

pub use connector::DatabaseOperationError;
pub use conversations::ConversationDbHandler;
pub use lumni::Timestamp;
pub use model::{ModelIdentifier, ModelSpec};
use serde::{Deserialize, Serialize};
pub use store::ConversationDatabase;
pub use user_profiles::UserProfileDbHandler;

pub use super::ConversationCache;
use super::PromptRole;
pub use crate::external as lumni;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversationStatus {
    Active,
    Archived,
    Deleted,
}

impl ConversationStatus {
    pub fn to_string(&self) -> String {
        match self {
            ConversationStatus::Active => "active".to_string(),
            ConversationStatus::Archived => "archived".to_string(),
            ConversationStatus::Deleted => "deleted".to_string(),
        }
    }
}

impl ConversationStatus {
    pub fn from_str(s: &str) -> Result<Self, ConversionError> {
        match s {
            "active" => Ok(ConversationStatus::Active),
            "archived" => Ok(ConversationStatus::Archived),
            "deleted" => Ok(ConversationStatus::Deleted),
            _ => Err(ConversionError::new("status", s)),
        }
    }
}

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
    pub message_count: Option<i64>,
    pub total_tokens: Option<i64>,
    pub is_deleted: bool,
    pub is_pinned: bool,
    pub status: ConversationStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub vote: i64,
    pub include_in_prompt: bool,
    pub is_hidden: bool,
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

#[derive(Debug)]
pub struct ConversionError {
    field: String,
    value: String,
}

impl ConversionError {
    pub fn new<T: fmt::Display>(field: &str, value: T) -> Self {
        ConversionError {
            field: field.to_string(),
            value: value.to_string(),
        }
    }
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid {} value: '{}'", self.field, self.value)
    }
}

impl Error for ConversionError {}
