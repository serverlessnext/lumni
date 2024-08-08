mod delete;
mod fetch;
mod insert;
mod update;

use std::sync::Arc;

use lumni::api::error::ApplicationError;
use rusqlite::{params, Error as SqliteError, OptionalExtension};
use serde_json::Value as JsonValue;
use tokio::sync::Mutex as TokioMutex;

use super::connector::{DatabaseConnector, DatabaseOperationError};
use super::encryption::EncryptionHandler;
use super::{
    Attachment, AttachmentData, AttachmentId, Conversation, ConversationId,
    ConversationStatus, Message, MessageId, ModelIdentifier, ModelSpec,
    Timestamp,
};
use crate::external as lumni;

#[derive(Clone)]
pub struct ConversationDbHandler {
    conversation_id: Option<ConversationId>,
    db: Arc<TokioMutex<DatabaseConnector>>,
    encryption_handler: Option<Arc<EncryptionHandler>>,
}

impl ConversationDbHandler {
    pub fn new(
        conversation_id: Option<ConversationId>,
        db: Arc<TokioMutex<DatabaseConnector>>,
        encryption_handler: Option<Arc<EncryptionHandler>>,
    ) -> Self {
        ConversationDbHandler {
            conversation_id,
            db,
            encryption_handler,
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
