use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Error as SqliteError;

mod connector;
mod schema;
mod store;

pub use schema::{
    Attachment, Conversation, ConversationCache, ConversationId, Exchange,
    ExchangeId, Message, ModelId,
};
pub use store::ConversationDatabaseStore;

pub use super::PromptRole;

pub struct ConversationDatabase {
    pub store: Arc<Mutex<ConversationDatabaseStore>>,
}

impl ConversationDatabase {
    pub fn new(sqlite_file: &PathBuf) -> Result<Self, SqliteError> {
        Ok(Self {
            store: Arc::new(Mutex::new(ConversationDatabaseStore::new(
                sqlite_file,
            )?)),
        })
    }

    pub fn new_conversation(
        &self,
        name: &str,
        parent_id: Option<ConversationId>,
    ) -> Result<ConversationId, SqliteError> {
        let mut store = self.store.lock().unwrap();
        let conversation = Conversation {
            id: ConversationId(-1), // Temporary ID
            name: name.to_string(),
            metadata: serde_json::Value::Null,
            parent_conversation_id: parent_id,
            fork_exchange_id: None,
            schema_version: 1,
            created_at: 0,
            updated_at: 0,
            is_deleted: false,
        };
        let conversation_id = store.put_new_conversation(&conversation)?;
        Ok(conversation_id)
    }

    pub fn finalize_exchange(
        &self,
        exchange: &Exchange,
        cache: &ConversationCache,
    ) -> Result<(), SqliteError> {
        let messages = cache.get_exchange_messages(exchange.id);
        let attachments = messages
            .iter()
            .flat_map(|message| cache.get_message_attachments(message.id))
            .collect::<Vec<_>>();
        let owned_messages: Vec<Message> =
            messages.into_iter().cloned().collect();
        let owned_attachments: Vec<Attachment> =
            attachments.into_iter().cloned().collect();
        let mut store = self.store.lock().unwrap();
        store.put_finalized_exchange(
            exchange,
            &owned_messages,
            &owned_attachments,
        )?;
        Ok(())
    }

    pub fn get_recent_conversations_with_last_exchange_and_messages(
        &self,
        limit: usize,
    ) -> Result<
        Vec<(Conversation, Option<(Exchange, Vec<Message>)>)>,
        SqliteError,
    > {
        let mut store = self.store.lock().unwrap();
        store.get_recent_conversations_with_last_exchange_and_messages(limit)
    }
}
