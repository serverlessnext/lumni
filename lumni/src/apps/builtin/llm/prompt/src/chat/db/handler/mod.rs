mod delete;
mod fetch;
mod insert;
mod update;

use std::sync::{Arc, Mutex};

use rusqlite::{params, Error as SqliteError, OptionalExtension};
use tokio::sync::Mutex as TokioMutex;

use super::connector::DatabaseConnector;
use super::{
    Attachment, AttachmentData, AttachmentId, Conversation, ConversationId,
    ConversationStatus, Message, MessageId, ModelIdentifier, ModelSpec,
    Timestamp,
};

#[derive(Clone)]
pub struct ConversationDbHandler {
    conversation_id: Option<ConversationId>,
    db: Arc<TokioMutex<DatabaseConnector>>,
}

impl ConversationDbHandler {
    pub fn new(
        conversation_id: Option<ConversationId>,
        db: Arc<TokioMutex<DatabaseConnector>>,
    ) -> Self {
        ConversationDbHandler {
            conversation_id,
            db,
        }
    }
}

impl ConversationDbHandler {
    pub fn get_conversation_id(&self) -> Option<ConversationId> {
        self.conversation_id
    }

    pub fn set_conversation_id(&mut self, conversation_id: ConversationId) {
        self.conversation_id = Some(conversation_id);
    }
}
