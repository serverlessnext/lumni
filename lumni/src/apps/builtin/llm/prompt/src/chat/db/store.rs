use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use lumni::api::error::ApplicationError;
use rusqlite::{Error as SqliteError, OptionalExtension};
use tokio::sync::Mutex as TokioMutex;

use super::connector::{DatabaseConnector, DatabaseOperationError};
use super::conversations::ConversationDbHandler;
use super::encryption::EncryptionHandler;
use super::user_profile::{UserProfile, UserProfileDbHandler};
use super::{
    Conversation, ConversationId, ConversationStatus, Message, MessageId,
    ModelIdentifier, Workspace, WorkspaceId,
};
use crate::external as lumni;

static PROMPT_SQLITE_FILEPATH: OnceLock<PathBuf> = OnceLock::new();

pub struct ConversationDatabase {
    db: Arc<TokioMutex<DatabaseConnector>>,
    encryption_handler: Option<Arc<EncryptionHandler>>,
}

impl ConversationDatabase {
    pub fn new(
        sqlite_file: &PathBuf,
        encryption_handler: Option<Arc<EncryptionHandler>>,
    ) -> Result<Self, DatabaseOperationError> {
        PROMPT_SQLITE_FILEPATH
            .set(sqlite_file.clone())
            .map_err(|_| {
                DatabaseOperationError::ApplicationError(
                    ApplicationError::DatabaseError(
                        "Failed to set the SQLite filepath".to_string(),
                    ),
                )
            })?;
        Ok(Self {
            db: Arc::new(TokioMutex::new(DatabaseConnector::new(sqlite_file)?)),
            encryption_handler,
        })
    }

    pub fn get_filepath() -> &'static PathBuf {
        PROMPT_SQLITE_FILEPATH.get().expect("Filepath not set")
    }

    pub fn get_conversation_handler(
        &self,
        conversation_id: Option<ConversationId>,
    ) -> ConversationDbHandler {
        ConversationDbHandler::new(
            conversation_id,
            self.db.clone(),
            self.encryption_handler.clone(),
        )
    }

    pub fn get_profile_handler(
        &self,
        profile: Option<UserProfile>,
    ) -> UserProfileDbHandler {
        UserProfileDbHandler::new(
            profile,
            self.db.clone(),
            self.encryption_handler.clone(),
        )
    }

    pub async fn truncate_and_vacuum(&self) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            // Disable foreign key constraints temporarily
            tx.execute("PRAGMA foreign_keys = OFF", [])?;

            // Truncate all tables except metadata, user_profiles, and models
            tx.execute_batch(
                "
                DELETE FROM conversation_tags;
                DELETE FROM tags;
                DELETE FROM attachments;
                DELETE FROM messages;
                DELETE FROM conversations;
            ",
            )?;

            // Re-enable foreign key constraints
            tx.execute("PRAGMA foreign_keys = ON", [])?;
            Ok(())
        })?;
        db.vacuum()?; // Reclaim unused space
        Ok(())
    }

    pub async fn fetch_last_conversation_id(
        &self,
    ) -> Result<Option<ConversationId>, DatabaseOperationError> {
        let query = "SELECT id FROM conversations WHERE is_deleted = FALSE \
                     ORDER BY updated_at DESC LIMIT 1";
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let result: Result<Option<ConversationId>, SqliteError> = tx
                .query_row(query, [], |row| Ok(ConversationId(row.get(0)?)))
                .optional();
            result.map_err(DatabaseOperationError::from)
        })
    }

    pub async fn fetch_conversation_with_messages(
        &self,
        conversation_id: Option<ConversationId>,
        limit: Option<usize>,
    ) -> Result<Option<(Conversation, Vec<Message>)>, DatabaseOperationError>
    {
        let query = format!(
            "WITH target_conversation AS (
                SELECT id
                FROM conversations
                WHERE is_deleted = FALSE
                {}
                ORDER BY updated_at DESC
                LIMIT 1
            )
            SELECT c.*, w.name AS workspace_name, w.path AS workspace_path,
            m.id as message_id, m.role, m.message_type, 
            m.content, m.has_attachments, m.token_length, 
            m.previous_message_id, m.created_at as message_created_at,
            m.vote, m.include_in_prompt, m.is_hidden, m.is_deleted
            FROM target_conversation tc
            JOIN conversations c ON c.id = tc.id
            LEFT JOIN workspaces w ON c.workspace_id = w.id
            LEFT JOIN messages m ON c.id = m.conversation_id
            ORDER BY m.created_at ASC
            {}",
            conversation_id
                .map_or(String::new(), |id| format!("AND id = {}", id.0)),
            limit.map_or(String::new(), |l| format!("LIMIT {}", l))
        );
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let result: Result<
                Option<(Conversation, Vec<Message>)>,
                SqliteError,
            > = {
                let mut stmt = tx.prepare(&query)?;
                let rows = stmt.query_map([], |row| {
                    let conversation = Conversation {
                        id: ConversationId(row.get(0)?),
                        name: row.get(1)?,
                        info: serde_json::from_str(&row.get::<_, String>(2)?)
                            .unwrap_or_default(),
                        model_identifier: ModelIdentifier(row.get(3)?),
                        workspace: row.get::<_, Option<i64>>(4)?.and_then(
                            |id| {
                                let name =
                                    row.get::<_, Option<String>>(15).ok()?;
                                let path =
                                    row.get::<_, Option<String>>(16).ok()?;
                                Some(Workspace {
                                    id: WorkspaceId(id),
                                    name: name?,
                                    directory_path: path.map(PathBuf::from),
                                })
                            },
                        ),
                        parent_conversation_id: row
                            .get::<_, Option<i64>>(5)?
                            .map(ConversationId),
                        fork_message_id: row
                            .get::<_, Option<i64>>(6)?
                            .map(MessageId),
                        completion_options: row
                            .get::<_, Option<String>>(7)?
                            .map(|s| {
                                serde_json::from_str(&s).unwrap_or_default()
                            }),
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                        message_count: row.get(10)?,
                        total_tokens: row.get(11)?,
                        is_deleted: row.get::<_, i64>(12)? != 0,
                        is_pinned: row.get::<_, i64>(13)? != 0,
                        status: match row.get::<_, String>(14)?.as_str() {
                            "active" => ConversationStatus::Active,
                            "archived" => ConversationStatus::Archived,
                            "deleted" => ConversationStatus::Deleted,
                            _ => ConversationStatus::Active,
                        },
                    };
                    let message = if let Some(message_id) =
                        row.get::<_, Option<i64>>(17)?
                    {
                        Some(Message {
                            id: MessageId(message_id),
                            conversation_id: conversation.id,
                            role: row.get(18)?,
                            message_type: row.get(19)?,
                            content: row.get(20)?,
                            has_attachments: row.get::<_, i64>(21)? != 0,
                            token_length: row.get(22)?,
                            previous_message_id: row
                                .get::<_, Option<i64>>(23)?
                                .map(MessageId),
                            created_at: row.get(24)?,
                            vote: row.get(25)?,
                            include_in_prompt: row.get::<_, i64>(26)? != 0,
                            is_hidden: row.get::<_, i64>(27)? != 0,
                            is_deleted: row.get::<_, i64>(28)? != 0,
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
            };
            result.map_err(DatabaseOperationError::from)
        })
    }

    pub async fn fetch_conversation_list(
        &self,
        limit: usize,
    ) -> Result<Vec<Conversation>, DatabaseOperationError> {
        let reader = self.get_conversation_handler(None);
        reader
            .fetch_conversation_list(limit)
            .await
            .map_err(DatabaseOperationError::from)
    }
}
