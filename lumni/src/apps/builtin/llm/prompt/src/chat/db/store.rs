use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::{Error as SqliteError, OptionalExtension};
use tokio::sync::Mutex as TokioMutex;

use super::connector::DatabaseConnector;
use super::handler::ConversationDbHandler;
use super::{
    Conversation, ConversationId, ConversationStatus, Message, MessageId,
    ModelIdentifier,
};

pub struct ConversationDatabase {
    db: Arc<TokioMutex<DatabaseConnector>>,
}

impl ConversationDatabase {
    pub fn new(sqlite_file: &PathBuf) -> Result<Self, SqliteError> {
        Ok(Self {
            db: Arc::new(TokioMutex::new(DatabaseConnector::new(sqlite_file)?)),
        })
    }

    pub fn get_conversation_handler(
        &self,
        conversation_id: Option<ConversationId>,
    ) -> ConversationDbHandler {
        ConversationDbHandler::new(conversation_id, self.db.clone())
    }

    pub async fn fetch_last_conversation_id(
        &self,
    ) -> Result<Option<ConversationId>, SqliteError> {
        let query = "SELECT id FROM conversations WHERE is_deleted = FALSE \
                     ORDER BY updated_at DESC LIMIT 1";
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(query, [], |row| Ok(ConversationId(row.get(0)?)))
                .optional()
        })
    }

    pub async fn fetch_conversation(
        &self,
        conversation_id: Option<ConversationId>,
        limit: Option<usize>,
    ) -> Result<Option<(Conversation, Vec<Message>)>, SqliteError> {
        let query = format!(
            "WITH target_conversation AS (
                SELECT id
                FROM conversations
                WHERE is_deleted = FALSE
                {}
                ORDER BY updated_at DESC
                LIMIT 1
            )
            SELECT c.*, m.id as message_id, m.role, m.message_type, 
            m.content, m.has_attachments, m.token_length, 
            m.previous_message_id, m.created_at as message_created_at,
            m.vote, m.include_in_prompt, m.is_hidden, m.is_deleted
            FROM target_conversation tc
            JOIN conversations c ON c.id = tc.id
            LEFT JOIN messages m ON c.id = m.conversation_id
            ORDER BY m.created_at ASC
            {}",
            conversation_id
                .map_or(String::new(), |id| format!("AND id = {}", id.0)),
            limit.map_or(String::new(), |l| format!("LIMIT {}", l))
        );
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let mut stmt = tx.prepare(&query)?;
            let rows = stmt.query_map([], |row| {
                let conversation = Conversation {
                    id: ConversationId(row.get(0)?),
                    name: row.get(1)?,
                    info: serde_json::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or_default(),
                    model_identifier: ModelIdentifier(row.get(3)?),
                    parent_conversation_id: row.get(4).map(ConversationId).ok(),
                    fork_message_id: row.get(5).map(MessageId).ok(),
                    completion_options: row
                        .get::<_, Option<String>>(6)?
                        .map(|s| serde_json::from_str(&s).unwrap_or_default()),
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    message_count: row.get(9)?,
                    total_tokens: row.get(10)?,
                    is_deleted: row.get::<_, i64>(11)? != 0,
                    is_pinned: row.get::<_, i64>(12)? != 0,
                    status: match row.get::<_, String>(13)?.as_str() {
                        "active" => ConversationStatus::Active,
                        "archived" => ConversationStatus::Archived,
                        "deleted" => ConversationStatus::Deleted,
                        _ => ConversationStatus::Active,
                    },
                };
                let message = if !row.get::<_, Option<i64>>(14)?.is_none() {
                    Some(Message {
                        id: MessageId(row.get(14)?),
                        conversation_id: conversation.id,
                        role: row.get(15)?,
                        message_type: row.get(16)?,
                        content: row.get(17)?,
                        has_attachments: row.get::<_, i64>(18)? != 0,
                        token_length: row.get(19)?,
                        previous_message_id: row.get(20).map(MessageId).ok(),
                        created_at: row.get(21)?,
                        vote: row.get(22)?,
                        include_in_prompt: row.get::<_, i64>(23)? != 0,
                        is_hidden: row.get::<_, i64>(24)? != 0,
                        is_deleted: row.get::<_, i64>(25)? != 0,
                    })
                } else {
                    None
                };
                Ok((conversation, message))
            })?;
            let mut conversation = None;
            let mut messages = Vec::new();
            for row in rows {
                let (conv, message) = row?;
                if conversation.is_none() {
                    conversation = Some(conv);
                }
                if let Some(msg) = message {
                    messages.push(msg);
                }
            }
            Ok(conversation.map(|c| (c, messages)))
        })
    }

    pub async fn fetch_conversation_list(
        &self,
        limit: usize,
    ) -> Result<Vec<Conversation>, SqliteError> {
        let reader = self.get_conversation_handler(None);
        reader.fetch_conversation_list(limit).await
    }
}
