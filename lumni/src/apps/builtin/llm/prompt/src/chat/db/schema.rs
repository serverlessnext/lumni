use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::store::ConversationDatabaseStore;
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
    pub schema_version: i32,
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
    pub completion_tokens: Option<i32>,
    pub prompt_tokens: Option<i32>,
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
    pub token_length: Option<i32>,
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
        db_store: &Arc<Mutex<ConversationDatabaseStore>>,
        parent_id: Option<ConversationId>,
    ) -> ConversationId {
        let new_id = self.new_conversation_id();
        let conversation = Conversation {
            id: new_id,
            name: name.to_string(),
            metadata: serde_json::Value::Null,
            parent_conversation_id: parent_id,
            fork_exchange_id: None,
            schema_version: 1,
            created_at: 0, // not using timestamps for now, stick with 0 for now
            updated_at: 0, // not using timestamps for now, stick with 0 for now
            is_deleted: false,
        };
        self.add_conversation(db_store, conversation);
        new_id
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.insert(model.model_id, model);
    }

    pub fn add_conversation(
        &mut self,
        db_store: &Arc<Mutex<ConversationDatabaseStore>>,
        conversation: Conversation,
    ) {
        let mut store_lock = db_store.lock().unwrap();
        store_lock.store_new_conversation(&conversation);
        let result = store_lock.commit_queued_operations();
        eprintln!("Commit result: {:?}", result);

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
        if let Some(existing_message) =
            self.messages.get_mut(&updated_message.id)
        {
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
            message.token_length = new_token_length;
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

    pub fn finalize_last_exchange(
        &mut self,
        db_store: &Arc<Mutex<ConversationDatabaseStore>>,
        conversation_id: ConversationId,
    ) {
        let exchange = self.get_last_exchange(conversation_id);
        if let Some(exchange) = exchange {
            let messages = self.get_exchange_messages(exchange.id);
            let attachments = messages
                .iter()
                .flat_map(|message| {
                    self.get_message_attachments(message.id)
                })
                .collect::<Vec<_>>();

            // Convert Vec<&Message> to Vec<Message> and Vec<&Attachment> to Vec<Attachment>
            let owned_messages: Vec<Message> = messages.into_iter().cloned().collect();
            let owned_attachments: Vec<Attachment> = attachments.into_iter().cloned().collect();

            eprintln!("Owned messages: {:?}", owned_messages);

            let mut db_lock_store = db_store.lock().unwrap();
            db_lock_store.store_finalized_exchange(&exchange, &owned_messages, &owned_attachments);
            let result = db_lock_store.commit_queued_operations();
            eprintln!("Commit result: {:?}", result);
        }

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
