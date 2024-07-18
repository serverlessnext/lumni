use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::PromptRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachmentId(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub model_id: ModelId,
    pub model_name: String,
    pub model_service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub name: String,
    pub metadata: serde_json::Value,
    pub model_id: ModelId,
    pub parent_conversation_id: Option<ConversationId>,
    pub fork_message_id: Option<MessageId>, // New field
    pub completion_options: Option<serde_json::Value>,
    pub schema_version: i64,
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
pub struct ConversationCache {
    conversation_id: ConversationId,
    models: HashMap<ModelId, Model>,
    messages: Vec<Message>, // messages have to be ordered
    attachments: HashMap<AttachmentId, Attachment>,
    message_attachments: HashMap<MessageId, Vec<AttachmentId>>,
}

impl ConversationCache {
    pub fn new() -> Self {
        ConversationCache {
            conversation_id: ConversationId(0),
            models: HashMap::new(),
            messages: Vec::new(),
            attachments: HashMap::new(),
            message_attachments: HashMap::new(),
        }
    }

    pub fn get_conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    pub fn set_conversation_id(&mut self, conversation_id: ConversationId) {
        self.conversation_id = conversation_id;
    }

    pub fn new_message_id(&self) -> MessageId {
        MessageId(self.messages.len() as i64)
    }

    pub fn new_attachment_id(&self) -> AttachmentId {
        AttachmentId(self.attachments.len() as i64)
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.insert(model.model_id, model);
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn get_last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn get_last_message_id(&self) -> Option<MessageId> {
        self.get_last_message().map(|m| m.id)
    }

    pub fn get_message_by_id(&self, message_id: MessageId) -> Option<&Message> {
        self.messages.iter().find(|m| m.id == message_id)
    }

    pub fn get_conversation_messages(&self) -> Vec<&Message> {
        self.messages.iter().collect()
    }

    pub fn update_message_by_id(
        &mut self,
        message_id: MessageId,
        new_content: &str,
        new_token_length: Option<i64>,
    ) {
        if let Some(message) =
            self.messages.iter_mut().find(|m| m.id == message_id)
        {
            message.content = new_content.to_string();
            message.token_length = new_token_length;
        }
    }

    pub fn update_message_token_length(
        &mut self,
        message_id: &MessageId,
        new_token_length: i64,
    ) {
        if let Some(message) =
            self.messages.iter_mut().find(|m| m.id == *message_id)
        {
            message.token_length = Some(new_token_length);
        }
    }

    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.message_attachments
            .entry(attachment.message_id)
            .or_default()
            .push(attachment.attachment_id);
        self.attachments
            .insert(attachment.attachment_id, attachment);
    }

    pub fn get_message_attachments(
        &self,
        message_id: MessageId,
    ) -> Vec<&Attachment> {
        self.message_attachments
            .get(&message_id)
            .map(|attachment_ids| {
                attachment_ids
                    .iter()
                    .filter_map(|id| self.attachments.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_system_prompt(&self) -> Option<String> {
        // system prompt is the first message in the conversation
        self.messages
            .first()
            .filter(|m| m.role == PromptRole::System)
            .map(|m| m.content.clone())
    }
}
