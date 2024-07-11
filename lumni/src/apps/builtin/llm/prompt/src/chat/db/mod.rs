use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Error as SqliteError;

mod connector;
mod schema;
mod store;

pub use schema::{
    ConversationId, Exchange, InMemoryDatabase, Message, ModelId,
};
pub use store::ConversationDatabaseStore;

pub use super::PromptRole;

pub struct ConversationDatabase {
    pub store: Arc<Mutex<ConversationDatabaseStore>>,
    pub cache: Arc<Mutex<InMemoryDatabase>>,
}

impl ConversationDatabase {
    pub fn new(sqlite_file: &PathBuf) -> Result<Self, SqliteError> {
        Ok(Self {
            store: Arc::new(Mutex::new(ConversationDatabaseStore::new(
                sqlite_file,
            )?)),
            cache: Arc::new(Mutex::new(InMemoryDatabase::new())),
        })
    }
}
