use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use rusqlite;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub exchange_id: ExchangeId,
    pub role: Role,
    pub message_type: String,
    pub content: String,
    pub has_attachments: bool,
    pub token_length: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
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

pub struct InMemoryDatabase {
    models: HashMap<ModelId, Model>,
    conversations: HashMap<ConversationId, Conversation>,
    exchanges: HashMap<ExchangeId, Exchange>,
    messages: HashMap<MessageId, Message>,
    attachments: HashMap<AttachmentId, Attachment>,
    
    conversation_exchanges: HashMap<ConversationId, HashSet<ExchangeId>>,
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
            .insert(exchange.id);
        self.exchanges.insert(exchange.id, exchange);
    }

    pub fn add_message(&mut self, message: Message) {
        self.exchange_messages
            .entry(message.exchange_id)
            .or_default()
            .push(message.id);
        self.messages.insert(message.id, message);
    }

    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.message_attachments
            .entry(attachment.message_id)
            .or_default()
            .push(attachment.attachment_id);
        self.attachments.insert(attachment.attachment_id, attachment);
    }

    pub fn get_conversation_exchanges(&self, conversation_id: ConversationId) -> Vec<&Exchange> {
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

    pub fn get_exchange_messages(&self, exchange_id: ExchangeId) -> Vec<&Message> {
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

    pub fn get_message_attachments(&self, message_id: MessageId) -> Vec<&Attachment> {
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