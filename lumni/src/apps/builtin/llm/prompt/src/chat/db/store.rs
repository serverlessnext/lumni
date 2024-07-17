use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Error as SqliteError, OptionalExtension};

use super::connector::DatabaseConnector;
use super::schema::{
    Attachment, AttachmentData, Conversation, ConversationCache,
    ConversationId, Exchange, ExchangeId, Message, MessageId,
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

    pub fn new_conversation(
        &self,
        name: &str,
        parent_id: Option<ConversationId>,
    ) -> Result<ConversationId, SqliteError> {
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
        self.put_new_conversation(&conversation)
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
        self.put_finalized_exchange(
            exchange,
            &owned_messages,
            &owned_attachments,
        )?;
        Ok(())
    }

    pub fn put_new_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<ConversationId, SqliteError> {
        let conversation_sql = format!(
            "INSERT INTO conversations (name, metadata, \
             parent_conversation_id, fork_exchange_id, schema_version, \
             created_at, updated_at, is_deleted)
            VALUES ('{}', {}, {}, {}, {}, {}, {}, {});",
            conversation.name.replace("'", "''"),
            serde_json::to_string(&conversation.metadata)
                .map(|s| format!("'{}'", s.replace("'", "''")))
                .unwrap_or_else(|_| "NULL".to_string()),
            conversation
                .parent_conversation_id
                .map_or("NULL".to_string(), |id| id.0.to_string()),
            conversation
                .fork_exchange_id
                .map_or("NULL".to_string(), |id| id.0.to_string()),
            conversation.schema_version,
            conversation.created_at,
            conversation.updated_at,
            conversation.is_deleted
        );
        let mut db = self.db.lock().unwrap();
        db.queue_operation(conversation_sql);
        db.process_queue_with_result(|tx| {
            let id = tx.last_insert_rowid();
            Ok(ConversationId(id))
        })
    }

    pub fn put_finalized_exchange(
        &self,
        exchange: &Exchange,
        messages: &[Message],
        attachments: &[Attachment],
    ) -> Result<(), SqliteError> {
        let mut db = self.db.lock().unwrap();
        // Update the previous exchange to set is_latest to false
        let last_exchange_id: Option<i64> =
            db.process_queue_with_result(|tx| {
                tx.query_row(
                    "SELECT id FROM exchanges WHERE conversation_id = ? AND \
                     is_latest = TRUE LIMIT 1",
                    params![exchange.conversation_id.0],
                    |row| row.get(0),
                )
                .optional()
            })?;

        // Update the previous exchange to set is_latest to false
        if let Some(prev_id) = last_exchange_id {
            let update_prev_sql = format!(
                "UPDATE exchanges SET is_latest = FALSE WHERE id = {};",
                prev_id
            );
            db.queue_operation(update_prev_sql);
        }

        // Insert the exchange (without token-related fields)
        let exchange_sql = format!(
            "INSERT INTO exchanges (conversation_id, model_id, system_prompt, 
         completion_options, prompt_options, created_at, previous_exchange_id, \
             is_deleted, is_latest)
        VALUES ({}, {}, {}, {}, {}, {}, {}, {}, TRUE);",
            exchange.conversation_id.0,
            exchange.model_id.0,
            exchange.system_prompt.as_ref().map_or(
                "NULL".to_string(),
                |s| format!("'{}'", s.replace("'", "''"))
            ),
            exchange.completion_options.as_ref().map_or(
                "NULL".to_string(),
                |v| format!("'{}'", v.to_string().replace("'", "''"))
            ),
            exchange.prompt_options.as_ref().map_or(
                "NULL".to_string(),
                |v| format!("'{}'", v.to_string().replace("'", "''"))
            ),
            exchange.created_at,
            last_exchange_id.map_or("NULL".to_string(), |id| id.to_string()),
            exchange.is_deleted
        );
        db.queue_operation(exchange_sql);

        // Get the actual exchange_id from the database
        let exchange_id = db.process_queue_with_result(|tx| {
            Ok(ExchangeId(tx.last_insert_rowid()))
        })?;

        // Insert messages and calculate total token length
        let mut total_tokens = 0;
        for message in messages {
            let message_sql = format!(
                "INSERT INTO messages (conversation_id, exchange_id, role, 
             message_type, content, has_attachments, token_length, 
             created_at, is_deleted)
            VALUES ({}, {}, '{}', '{}', '{}', {}, {}, {}, {});",
                exchange.conversation_id.0,
                exchange_id.0,
                message.role.to_string(),
                message.message_type,
                message.content.replace("'", "''"),
                message.has_attachments,
                message
                    .token_length
                    .map_or("NULL".to_string(), |t| t.to_string()),
                message.created_at,
                message.is_deleted
            );
            db.queue_operation(message_sql);

            // Sum up token lengths
            if let Some(token_length) = message.token_length {
                total_tokens += token_length;
            }
        }

        // Insert attachments
        for attachment in attachments {
            let attachment_sql = format!(
                "INSERT INTO attachments (message_id, conversation_id, 
             exchange_id, file_uri, file_data, file_type, metadata, 
             created_at, is_deleted)
            VALUES ({}, {}, {}, {}, {}, '{}', {}, {}, {});",
                attachment.message_id.0,
                exchange.conversation_id.0,
                exchange_id.0,
                match &attachment.data {
                    AttachmentData::Uri(uri) =>
                        format!("'{}'", uri.replace("'", "''")),
                    AttachmentData::Data(_) => "NULL".to_string(),
                },
                match &attachment.data {
                    AttachmentData::Uri(_) => "NULL".to_string(),
                    AttachmentData::Data(data) =>
                        format!("X'{}'", hex::encode(data)),
                },
                attachment.file_type,
                attachment.metadata.as_ref().map_or(
                    "NULL".to_string(),
                    |v| format!("'{}'", v.to_string().replace("'", "''"))
                ),
                attachment.created_at,
                attachment.is_deleted
            );
            db.queue_operation(attachment_sql);
        }

        // Update conversation
        let update_conversation_sql = format!(
            "UPDATE conversations 
        SET updated_at = {}, 
            exchange_count = exchange_count + 1,
            total_tokens = total_tokens + {}
        WHERE id = {};",
            exchange.created_at, total_tokens, exchange.conversation_id.0
        );
        db.queue_operation(update_conversation_sql);

        // Commit the transaction
        db.process_queue()?;
        Ok(())
    }

    pub fn fetch_recent_conversations(
        &self,
        limit: usize,
    ) -> Result<Vec<(Conversation, Option<Vec<Message>>)>, SqliteError> {
        let query = format!(
            "SELECT c.*, m.id as message_id, m.role, m.message_type, \
             m.content, m.has_attachments, m.token_length, m.created_at as \
             message_created_at
             FROM conversations c
             LEFT JOIN exchanges e ON c.id = e.conversation_id AND e.is_latest \
             = TRUE
             LEFT JOIN messages m ON e.id = m.exchange_id
             WHERE c.is_deleted = FALSE
             ORDER BY c.updated_at DESC, m.created_at ASC
             LIMIT {}",
            limit
        );

        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            let mut stmt = tx.prepare(&query)?;
            let rows = stmt.query_map([], |row| {
                let conversation = Conversation {
                    id: ConversationId(row.get(0)?),
                    name: row.get(1)?,
                    metadata: serde_json::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or_default(),
                    parent_conversation_id: row.get(3).map(ConversationId).ok(),
                    fork_exchange_id: row.get(4).map(ExchangeId).ok(),
                    schema_version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    is_deleted: row.get(10)?,
                };

                let message = if !row.get::<_, Option<i64>>(11)?.is_none() {
                    Some(Message {
                        id: MessageId(row.get(11)?),
                        conversation_id: conversation.id,
                        exchange_id: ExchangeId(row.get(0)?), // Using conversation_id as exchange_id
                        role: row.get(12)?,
                        message_type: row.get(13)?,
                        content: row.get(14)?,
                        has_attachments: row.get(15)?,
                        token_length: row.get(16)?,
                        created_at: row.get(17)?,
                        is_deleted: false,
                    })
                } else {
                    None
                };

                Ok((conversation, message))
            })?;

            let mut result = Vec::new();
            let mut current_conversation: Option<Conversation> = None;
            let mut current_messages = Vec::new();

            for row in rows {
                let (conversation, message) = row?;

                if current_conversation
                    .as_ref()
                    .map_or(true, |c| c.id != conversation.id)
                {
                    if let Some(conv) = current_conversation.take() {
                        result.push((
                            conv,
                            Some(std::mem::take(&mut current_messages)),
                        ));
                    }
                    current_conversation = Some(conversation);
                }

                if let Some(msg) = message {
                    current_messages.push(msg);
                }
            }

            if let Some(conv) = current_conversation.take() {
                result.push((conv, Some(current_messages)));
            }

            Ok(result)
        })
    }
}

impl ConversationDatabaseStore {
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
            m.content, m.has_attachments, m.token_length, m.created_at as \
             message_created_at
            FROM target_conversation tc
            JOIN conversations c ON c.id = tc.id
            LEFT JOIN exchanges e ON c.id = e.conversation_id
            LEFT JOIN messages m ON e.id = m.exchange_id
            ORDER BY e.created_at DESC, m.created_at ASC
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
                    metadata: serde_json::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or_default(),
                    parent_conversation_id: row.get(3).map(ConversationId).ok(),
                    fork_exchange_id: row.get(4).map(ExchangeId).ok(),
                    schema_version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    is_deleted: row.get(10)?,
                };

                let message = if !row.get::<_, Option<i64>>(11)?.is_none() {
                    Some(Message {
                        id: MessageId(row.get(11)?),
                        conversation_id: conversation.id,
                        exchange_id: ExchangeId(row.get(0)?),
                        role: row.get(12)?,
                        message_type: row.get(13)?,
                        content: row.get(14)?,
                        has_attachments: row.get(15)?,
                        token_length: row.get(16)?,
                        created_at: row.get(17)?,
                        is_deleted: false,
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
            "SELECT id, name, updated_at
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
                    updated_at: row.get(2)?,
                    // Set other fields to default values or None
                    metadata: serde_json::Value::Null,
                    parent_conversation_id: None,
                    fork_exchange_id: None,
                    schema_version: 1,
                    created_at: 0,
                    is_deleted: false,
                })
            })?;

            rows.collect()
        })
    }
}
