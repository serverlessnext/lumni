use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Error as SqliteError, OptionalExtension};

use super::connector::DatabaseConnector;
use super::helpers::system_time_in_milliseconds;
use super::reader::ConversationReader;
use super::{
    Attachment, AttachmentData, AttachmentId, Conversation, ConversationId,
    ConversationStatus, Message, MessageId, ModelIdentifier, ModelSpec,
};

pub struct ConversationDatabaseStore {
    db: Arc<Mutex<DatabaseConnector>>,
}

impl ConversationDatabaseStore {
    pub fn new(sqlite_file: &PathBuf) -> Result<Self, SqliteError> {
        Ok(Self {
            db: Arc::new(Mutex::new(DatabaseConnector::new(sqlite_file)?)),
        })
    }

    pub fn get_conversation_reader(
        &self,
        conversation_id: ConversationId,
    ) -> ConversationReader {
        ConversationReader::new(conversation_id, &self.db)
    }

    pub fn new_conversation(
        &self,
        name: &str,
        parent_id: Option<ConversationId>,
        fork_message_id: Option<MessageId>,
        completion_options: Option<serde_json::Value>,
        model: &ModelSpec,
    ) -> Result<ConversationId, SqliteError> {
        let mut db = self.db.lock().unwrap();

        let timestamp = system_time_in_milliseconds();

        db.process_queue_with_result(|tx| {
            // Ensure the model exists
            let exists: bool = tx
                .query_row(
                    "SELECT 1 FROM models WHERE identifier = ?",
                    params![model.identifier.0],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);

            if !exists {
                tx.execute(
                    "INSERT INTO models (identifier, info, config, \
                     context_window_size, input_token_limit)
                    VALUES (?, ?, ?, ?, ?)",
                    params![
                        model.identifier.0,
                        model
                            .info
                            .as_ref()
                            .map(|v| serde_json::to_string(v)
                                .unwrap_or_default()),
                        model
                            .config
                            .as_ref()
                            .map(|v| serde_json::to_string(v)
                                .unwrap_or_default()),
                        model.context_window_size,
                        model.input_token_limit,
                    ],
                )?;
            }

            // Create the conversation
            let conversation = Conversation {
                id: ConversationId(-1), // Temporary ID
                name: name.to_string(),
                info: serde_json::Value::Null,
                model_identifier: model.identifier.clone(),
                parent_conversation_id: parent_id,
                fork_message_id,
                completion_options,
                created_at: timestamp,
                updated_at: timestamp,
                is_deleted: false,
                status: ConversationStatus::Active,
            };

            tx.execute(
                "INSERT INTO conversations (
                    name, info, model_identifier, parent_conversation_id, 
                    fork_message_id, completion_options, created_at, \
                 updated_at, 
                    message_count, total_tokens, is_deleted, status
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, 0, ?, ?)",
                params![
                    conversation.name,
                    serde_json::to_string(&conversation.info)
                        .unwrap_or_default(),
                    conversation.model_identifier.0,
                    conversation.parent_conversation_id.map(|id| id.0),
                    conversation.fork_message_id.map(|id| id.0),
                    conversation
                        .completion_options
                        .as_ref()
                        .map(|v| serde_json::to_string(v).unwrap_or_default()),
                    conversation.created_at,
                    conversation.updated_at,
                    conversation.is_deleted,
                    match conversation.status {
                        ConversationStatus::Active => "active",
                        ConversationStatus::Archived => "archived",
                        ConversationStatus::Deleted => "deleted",
                    },
                ],
            )?;

            let id = tx.last_insert_rowid();
            Ok(ConversationId(id))
        })
    }

    pub fn update_conversation_status(
        &self,
        conversation_id: ConversationId,
        status: ConversationStatus,
    ) -> Result<(), SqliteError> {
        let query = "UPDATE conversations SET status = ? WHERE id = ?";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.execute(
                query,
                params![
                    match status {
                        ConversationStatus::Active => "active",
                        ConversationStatus::Archived => "archived",
                        ConversationStatus::Deleted => "deleted",
                    },
                    conversation_id.0
                ],
            )?;
            Ok(())
        })
    }

    pub fn add_conversation_tag(
        &self,
        conversation_id: ConversationId,
        tag: &str,
    ) -> Result<(), SqliteError> {
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?)",
                params![tag],
            )?;
            let tag_id: i64 = tx.query_row(
                "SELECT id FROM tags WHERE name = ?",
                params![tag],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO conversation_tags (conversation_id, \
                 tag_id) VALUES (?, ?)",
                params![conversation_id.0, tag_id],
            )?;
            Ok(())
        })
    }

    pub fn remove_conversation_tag(
        &self,
        conversation_id: ConversationId,
        tag: &str,
    ) -> Result<(), SqliteError> {
        let query = "
            DELETE FROM conversation_tags
            WHERE conversation_id = ? AND tag_id = (SELECT id FROM tags WHERE \
                     name = ?)
        ";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.execute(query, params![conversation_id.0, tag])?;
            Ok(())
        })
    }

    pub fn fetch_last_conversation_id(
        &self,
    ) -> Result<Option<ConversationId>, SqliteError> {
        let query = "SELECT id FROM conversations WHERE is_deleted = FALSE \
                     ORDER BY updated_at DESC LIMIT 1";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.query_row(query, [], |row| Ok(ConversationId(row.get(0)?)))
                .optional()
        })
    }

    pub fn fetch_conversation(
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

        let mut db = self.db.lock().unwrap();
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
                    is_deleted: row.get::<_, i64>(11)? != 0,
                    status: match row.get::<_, String>(12)?.as_str() {
                        "active" => ConversationStatus::Active,
                        "archived" => ConversationStatus::Archived,
                        "deleted" => ConversationStatus::Deleted,
                        _ => ConversationStatus::Active,
                    },
                };

                let message = if !row.get::<_, Option<i64>>(13)?.is_none() {
                    Some(Message {
                        id: MessageId(row.get(13)?),
                        conversation_id: conversation.id,
                        role: row.get(14)?,
                        message_type: row.get(15)?,
                        content: row.get(16)?,
                        has_attachments: row.get::<_, i64>(17)? != 0,
                        token_length: row.get(18)?,
                        previous_message_id: row.get(19).map(MessageId).ok(),
                        created_at: row.get(20)?,
                        vote: row.get(21)?,
                        include_in_prompt: row.get::<_, i64>(22)? != 0,
                        is_hidden: row.get::<_, i64>(23)? != 0,
                        is_deleted: row.get::<_, i64>(24)? != 0,
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

    pub fn fetch_conversation_list(
        &self,
        limit: usize,
    ) -> Result<Vec<Conversation>, SqliteError> {
        let query = format!(
            "SELECT id, name, info, completion_options, model_identifier, \
             parent_conversation_id, 
             fork_message_id, created_at, updated_at, is_deleted, status
             FROM conversations
             WHERE is_deleted = FALSE
             ORDER BY updated_at DESC
             LIMIT {}",
            limit
        );
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            let mut stmt = tx.prepare(&query)?;
            let rows = stmt.query_map([], |row| {
                Ok(Conversation {
                    id: ConversationId(row.get(0)?),
                    name: row.get(1)?,
                    info: serde_json::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or_default(),
                    completion_options: row
                        .get::<_, Option<String>>(3)?
                        .map(|s| serde_json::from_str(&s).unwrap_or_default()),
                    model_identifier: ModelIdentifier(row.get(4)?),
                    parent_conversation_id: row
                        .get::<_, Option<i64>>(5)?
                        .map(ConversationId),
                    fork_message_id: row
                        .get::<_, Option<i64>>(6)?
                        .map(MessageId),
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    is_deleted: row.get::<_, i64>(9)? != 0,
                    status: ConversationStatus::from_str(
                        &row.get::<_, String>(10)?,
                    )
                    .unwrap_or(ConversationStatus::Active),
                })
            })?;
            rows.collect()
        })
    }
}

impl ConversationDatabaseStore {
    pub fn put_new_message(
        &self,
        message: &Message,
    ) -> Result<MessageId, SqliteError> {
        let mut db = self.db.lock().unwrap();

        db.process_queue_with_result(|tx| {
            // Get the last message ID for this conversation
            let last_message_id: Option<i64> = tx
                .query_row(
                    "SELECT id FROM messages WHERE conversation_id = ? ORDER \
                     BY id DESC LIMIT 1",
                    params![message.conversation_id.0],
                    |row| row.get(0),
                )
                .optional()?;

            // Insert the new message
            tx.execute(
                "INSERT INTO messages (
                    conversation_id, role, message_type, content, 
                    has_attachments, token_length, previous_message_id, 
                    created_at, vote, include_in_prompt, is_hidden, is_deleted
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    message.conversation_id.0,
                    message.role.to_string(),
                    message.message_type,
                    message.content,
                    message.has_attachments,
                    message.token_length,
                    last_message_id,
                    message.created_at,
                    message.vote,
                    message.include_in_prompt,
                    message.is_hidden,
                    message.is_deleted
                ],
            )?;

            let new_message_id = MessageId(tx.last_insert_rowid());

            // Update the conversation
            tx.execute(
                "UPDATE conversations 
                SET updated_at = ?, 
                    message_count = message_count + 1,
                    total_tokens = total_tokens + ?
                WHERE id = ?",
                params![
                    message.created_at,
                    message.token_length.unwrap_or(0),
                    message.conversation_id.0
                ],
            )?;

            Ok(new_message_id)
        })
    }

    pub fn put_new_messages(
        &self,
        messages: &[Message],
    ) -> Result<Vec<MessageId>, SqliteError> {
        if messages.is_empty() {
            return Ok(vec![]);
        }

        let mut db = self.db.lock().unwrap();

        db.process_queue_with_result(|tx| {
            let conversation_id = messages[0].conversation_id.0;
            let mut new_message_ids = Vec::with_capacity(messages.len());
            let mut last_message_id: Option<i64> = tx
                .query_row(
                    "SELECT id FROM messages WHERE conversation_id = ? ORDER \
                     BY id DESC LIMIT 1",
                    params![conversation_id],
                    |row| row.get(0),
                )
                .optional()?;

            for message in messages {
                tx.execute(
                    "INSERT INTO messages (
                        conversation_id, role, message_type, content, 
                        has_attachments, token_length, previous_message_id, 
                        created_at, vote, include_in_prompt, is_hidden, \
                     is_deleted
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        message.conversation_id.0,
                        message.role.to_string(),
                        message.message_type,
                        message.content,
                        message.has_attachments,
                        message.token_length,
                        last_message_id,
                        message.created_at,
                        message.vote,
                        message.include_in_prompt,
                        message.is_hidden,
                        message.is_deleted
                    ],
                )?;

                let new_id = tx.last_insert_rowid();
                new_message_ids.push(MessageId(new_id));
                last_message_id = Some(new_id);
            }

            // Update conversation
            tx.execute(
                "UPDATE conversations 
                SET updated_at = ?, 
                    message_count = message_count + ?,
                    total_tokens = total_tokens + ?
                WHERE id = ?",
                params![
                    messages.last().map_or(0, |m| m.created_at),
                    messages.len() as i64,
                    messages.iter().filter_map(|m| m.token_length).sum::<i64>(),
                    conversation_id
                ],
            )?;

            Ok(new_message_ids)
        })
    }

    pub fn fetch_message_attachments(
        &self,
        message_id: MessageId,
    ) -> Result<Vec<Attachment>, SqliteError> {
        let query = "SELECT * FROM attachments WHERE message_id = ? AND \
                     is_deleted = FALSE";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            let mut stmt = tx.prepare(query)?;
            let rows = stmt.query_map(params![message_id.0], |row| {
                Ok(Attachment {
                    attachment_id: AttachmentId(row.get(0)?),
                    message_id: MessageId(row.get(1)?),
                    conversation_id: ConversationId(row.get(2)?),
                    data: if let Some(uri) = row.get::<_, Option<String>>(3)? {
                        AttachmentData::Uri(uri)
                    } else {
                        AttachmentData::Data(row.get(4)?)
                    },
                    file_type: row.get(5)?,
                    metadata: row
                        .get::<_, Option<String>>(6)?
                        .map(|s| serde_json::from_str(&s).unwrap_or_default()),
                    created_at: row.get(7)?,
                    is_deleted: row.get(8)?,
                })
            })?;
            rows.collect()
        })
    }
}
