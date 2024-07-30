mod delete;
mod fetch;
mod insert;
mod update;

use std::sync::{Arc, Mutex};

use rusqlite::{params, Error as SqliteError, OptionalExtension};

use super::connector::DatabaseConnector;
use super::{
    Attachment, AttachmentData, AttachmentId, Conversation, ConversationId,
    ConversationStatus, Message, MessageId, ModelIdentifier, ModelSpec,
    Timestamp,
};

pub struct ConversationDbHandler<'a> {
    conversation_id: Option<ConversationId>,
    db: &'a Arc<Mutex<DatabaseConnector>>,
}

impl<'a> ConversationDbHandler<'a> {
    pub fn new(
        conversation_id: Option<ConversationId>,
        db: &'a Arc<Mutex<DatabaseConnector>>,
    ) -> Self {
        ConversationDbHandler {
            conversation_id,
            db,
        }
    }
}

impl<'a> ConversationDbHandler<'a> {
    pub fn get_conversation_id(&self) -> Option<ConversationId> {
        self.conversation_id
    }

    pub fn set_conversation_id(&mut self, conversation_id: ConversationId) {
        self.conversation_id = Some(conversation_id);
    }
}
