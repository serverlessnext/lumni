use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use rusqlite;
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
    pub parent_conversation_id: ConversationId,
    pub fork_exchange_id: ExchangeId,
    pub schema_version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub id: ExchangeId,
    pub conversation_id: ConversationId,
    pub model_id: ModelId,
    pub system_prompt: String,
    pub completion_options: serde_json::Value,
    pub prompt_options: serde_json::Value,
    pub completion_tokens: i32,
    pub prompt_tokens: i32,
    pub created_at: i64,
    pub previous_exchange_id: Option<ExchangeId>,
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
    pub token_length: i32,
    pub created_at: i64,
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
    pub data: AttachmentData,
    pub file_type: String,
    pub metadata: serde_json::Value,
    pub created_at: i64,
}

impl Conversation {
    pub fn new(name: &str) -> Self {
        Conversation {
            id: ConversationId(0), // You might want to generate a unique ID here
            name: name.to_string(),
            metadata: serde_json::Value::Null,
            parent_conversation_id: ConversationId(0),
            fork_exchange_id: ExchangeId(0),
            schema_version: 1,
            created_at: 0, // not using timestamps for now, stick with 0 for now
            updated_at: 0, // not using timestamps for now, stick with 0 for now
        }
    }
}

#[derive(Debug)]
pub struct InMemoryDatabase {
    models: HashMap<ModelId, Model>,
    conversations: HashMap<ConversationId, Conversation>,
    exchanges: HashMap<ExchangeId, Exchange>,
    messages: HashMap<MessageId, Message>,
    attachments: HashMap<AttachmentId, Attachment>,

    conversation_exchanges: HashMap<ConversationId, Vec<ExchangeId>>,
    exchange_messages: HashMap<ExchangeId, Vec<MessageId>>,
    message_attachments: HashMap<MessageId, Vec<AttachmentId>>,
}

impl InMemoryDatabase {
    pub fn new() -> Self {
        InMemoryDatabase {
            models: HashMap::new(),
            conversations: HashMap::new(),
            exchanges: HashMap::new(),
            messages: HashMap::new(),
            attachments: HashMap::new(),
            conversation_exchanges: HashMap::new(),
            exchange_messages: HashMap::new(),
            message_attachments: HashMap::new(),
        }
    }

    pub fn new_conversation_id(&self) -> ConversationId {
        ConversationId(self.conversations.len() as i64)
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

    pub fn new_conversation(
        &mut self,
        name: &str,
        parent_id: Option<ConversationId>,
    ) -> ConversationId {
        let new_id = self.new_conversation_id();
        let mut conversation = Conversation::new(name);
        conversation.id = new_id;
        if let Some(parent) = parent_id {
            conversation.parent_conversation_id = parent;
        }
        self.add_conversation(conversation);
        new_id
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.insert(model.model_id, model);
    }

    pub fn add_conversation(&mut self, conversation: Conversation) {
        self.conversations.insert(conversation.id, conversation);
    }

    pub fn add_exchange(&mut self, exchange: Exchange) {
        self.conversation_exchanges
            .entry(exchange.conversation_id)
            .or_default()
            .push(exchange.id);
        self.exchanges.insert(exchange.id, exchange);
    }

    pub fn add_message(&mut self, message: Message) {
        self.exchange_messages
            .entry(message.exchange_id)
            .or_default()
            .push(message.id);
        self.messages.insert(message.id, message);
    }

    pub fn update_message(&mut self, updated_message: Message) {
        if let Some(existing_message) = self.messages.get_mut(&updated_message.id) {
            *existing_message = updated_message;
        }
    }

    pub fn update_message_by_id(
        &mut self,
        message_id: MessageId,
        new_content: &str,
        new_token_length: Option<i32>,
    ) {
        if let Some(message) = self.messages.get_mut(&message_id) {
            message.content = new_content.to_string();
            if let Some(token_length) = new_token_length {
                message.token_length = token_length;
            }
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

    pub fn get_conversation_exchanges(
        &self,
        conversation_id: ConversationId,
    ) -> Vec<&Exchange> {
        self.conversation_exchanges
            .get(&conversation_id)
            .map(|exchange_ids| {
                exchange_ids
                    .iter()
                    .filter_map(|id| self.exchanges.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_last_exchange(
        &self,
        conversation_id: ConversationId,
    ) -> Option<Exchange> {
        self.conversation_exchanges
            .get(&conversation_id)
            .and_then(|exchanges| exchanges.last())
            .and_then(|last_exchange_id| {
                self.exchanges.get(last_exchange_id).cloned()
            })
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

pub struct Database {
    in_memory: Arc<Mutex<InMemoryDatabase>>,
    sqlite_conn: Arc<Mutex<rusqlite::Connection>>,
}

impl Database {
    pub fn new(sqlite_path: &str) -> rusqlite::Result<Self> {
        let sqlite_conn = rusqlite::Connection::open(sqlite_path)?;
        Ok(Database {
            in_memory: Arc::new(Mutex::new(InMemoryDatabase::new())),
            sqlite_conn: Arc::new(Mutex::new(sqlite_conn)),
        })
    }

    pub fn save_in_background(&self) {
        let in_memory = Arc::clone(&self.in_memory);
        let sqlite_conn = Arc::clone(&self.sqlite_conn);

        thread::spawn(move || {
            let data = in_memory.lock().unwrap();
            let conn = sqlite_conn.lock().unwrap();
            // saving to SQLite here
        });
    }

    pub fn add_model(&self, model: Model) {
        let mut data = self.in_memory.lock().unwrap();
        data.add_model(model);
    }

    pub fn add_conversation(&self, conversation: Conversation) {
        let mut data = self.in_memory.lock().unwrap();
        data.add_conversation(conversation);
    }

    pub fn add_exchange(&self, exchange: Exchange) {
        let mut data = self.in_memory.lock().unwrap();
        data.add_exchange(exchange);
    }

    pub fn add_message(&self, message: Message) {
        let mut data = self.in_memory.lock().unwrap();
        data.add_message(message);
    }

    pub fn add_attachment(&self, attachment: Attachment) {
        let mut data = self.in_memory.lock().unwrap();
        data.add_attachment(attachment);
    }
}
