use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::PromptRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExchangeId(pub i64);

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
    pub parent_conversation_id: Option<ConversationId>,
    pub fork_exchange_id: Option<ExchangeId>,
    pub schema_version: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub id: ExchangeId,
    pub conversation_id: ConversationId,
    pub model_id: ModelId,
    pub system_prompt: Option<String>,
    pub completion_options: Option<serde_json::Value>,
    pub prompt_options: Option<serde_json::Value>,
    pub completion_tokens: Option<i64>,
    pub prompt_tokens: Option<i64>,
    pub created_at: i64,
    pub previous_exchange_id: Option<ExchangeId>,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub exchange_id: ExchangeId,
    pub role: PromptRole,
    pub message_type: String,
    pub content: String,
    pub has_attachments: bool,
    pub token_length: Option<i64>,
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
    pub exchange_id: ExchangeId,
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
    exchanges: Vec<Exchange>,
    messages: HashMap<MessageId, Message>,
    attachments: HashMap<AttachmentId, Attachment>,
    exchange_messages: HashMap<ExchangeId, Vec<MessageId>>,
    message_attachments: HashMap<MessageId, Vec<AttachmentId>>,
}

impl ConversationCache {
    pub fn new() -> Self {
        ConversationCache {
            conversation_id: ConversationId(0),
            models: HashMap::new(),
            exchanges: Vec::new(),
            messages: HashMap::new(),
            attachments: HashMap::new(),
            exchange_messages: HashMap::new(),
            message_attachments: HashMap::new(),
        }
    }

    pub fn get_conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    pub fn set_conversation_id(&mut self, conversation_id: ConversationId) {
        self.conversation_id = conversation_id;
    }

    pub fn new_exchange_id(&self) -> ExchangeId {
        ExchangeId(self.exchanges.len() as i64)
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

    pub fn add_exchange(&mut self, exchange: Exchange) {
        self.exchanges.push(exchange);
    }

    pub fn get_exchanges(&self) -> Vec<&Exchange> {
        self.exchanges.iter().collect()
    }

    pub fn add_message(&mut self, message: Message) {
        self.exchange_messages
            .entry(message.exchange_id)
            .or_default()
            .push(message.id);
        self.messages.insert(message.id, message);
    }

    pub fn update_message_by_id(
        &mut self,
        message_id: MessageId,
        new_content: &str,
        new_token_length: Option<i64>,
    ) {
        if let Some(message) = self.messages.get_mut(&message_id) {
            message.content = new_content.to_string();
            message.token_length = new_token_length;
        }
    }

    pub fn update_message_token_length(
        &mut self,
        message_id: MessageId,
        new_token_length: i64,
    ) {
        if let Some(message) = self.messages.get_mut(&message_id) {
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

    pub fn get_last_exchange(&self) -> Option<&Exchange> {
        self.exchanges.last()
    }

    pub fn get_exchange_messages(
        &self,
        exchange_id: ExchangeId,
    ) -> Vec<&Message> {
        self.exchange_messages
            .get(&exchange_id)
            .map(|message_ids| {
                message_ids
                    .iter()
                    .filter_map(|id| self.messages.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_last_message_of_exchange(
        &self,
        exchange_id: ExchangeId,
    ) -> Option<&Message> {
        self.exchange_messages
            .get(&exchange_id)
            .and_then(|messages| messages.last())
            .and_then(|last_message_id| self.messages.get(last_message_id))
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
}
