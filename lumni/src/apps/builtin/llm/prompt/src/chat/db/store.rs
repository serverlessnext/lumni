use std::path::PathBuf;

use rusqlite::{params, Error as SqliteError, OptionalExtension};

use super::connector::DatabaseConnector;
use super::schema::{
    Attachment, AttachmentData, Conversation, ConversationId, Exchange,
    ExchangeId, Message, MessageId, ModelId,
};

pub struct ConversationDatabaseStore {
    db: DatabaseConnector,
}

impl ConversationDatabaseStore {
    pub fn new(sqlite_file: &PathBuf) -> Result<Self, SqliteError> {
        Ok(Self {
            db: DatabaseConnector::new(sqlite_file)?,
        })
    }

    pub fn put_new_conversation(
        &mut self,
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

        self.db.queue_operation(conversation_sql);

        self.db.process_queue_with_result(|tx| {
            let id = tx.last_insert_rowid();
            Ok(ConversationId(id))
        })
    }

    pub fn put_finalized_exchange(
        &mut self,
        exchange: &Exchange,
        messages: &[Message],
        attachments: &[Attachment],
    ) -> Result<(), SqliteError> {
        // Update the previous exchange to set is_latest to false
        let last_exchange_id: Option<i64> =
            self.db.process_queue_with_result(|tx| {
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
            self.db.queue_operation(update_prev_sql);
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
        self.db.queue_operation(exchange_sql);

        // Get the actual exchange_id from the database
        let exchange_id = self.db.process_queue_with_result(|tx| {
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
            self.db.queue_operation(message_sql);

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
            self.db.queue_operation(attachment_sql);
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
        self.db.queue_operation(update_conversation_sql);

        // Commit the transaction
        self.commit_queued_operations()?;

        Ok(())
    }

    pub fn get_recent_conversations_with_last_exchange_and_messages(
        &mut self,
        limit: usize,
    ) -> Result<
        Vec<(Conversation, Option<(Exchange, Vec<Message>)>)>,
        SqliteError,
    > {
        let query = format!(
            "SELECT c.*, e.*, m.id as message_id, m.role, m.message_type, \
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

        self.db.process_queue_with_result(|tx| {
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

                let exchange = if !row.get::<_, Option<i64>>(11)?.is_none() {
                    Some(Exchange {
                        id: ExchangeId(row.get(11)?),
                        conversation_id: ConversationId(row.get(12)?),
                        model_id: ModelId(row.get(13)?),
                        system_prompt: row.get(14)?,
                        completion_options: row
                            .get::<_, Option<String>>(15)?
                            .map(|s| {
                                serde_json::from_str(&s).unwrap_or_default()
                            }),
                        prompt_options: row.get::<_, Option<String>>(16)?.map(
                            |s| serde_json::from_str(&s).unwrap_or_default(),
                        ),
                        created_at: row.get(17)?,
                        previous_exchange_id: row.get(18).map(ExchangeId).ok(),
                        is_deleted: row.get(19)?,
                    })
                } else {
                    None
                };

                let message = if !row.get::<_, Option<i64>>(21)?.is_none() {
                    Some(Message {
                        id: MessageId(row.get(21)?),
                        conversation_id: conversation.id,
                        exchange_id: exchange.as_ref().unwrap().id,
                        role: row.get(22)?,
                        message_type: row.get(23)?,
                        content: row.get(24)?,
                        has_attachments: row.get(25)?,
                        token_length: row.get(26)?,
                        created_at: row.get(27)?,
                        is_deleted: false,
                    })
                } else {
                    None
                };

                Ok((conversation, exchange, message))
            })?;

            let mut result = Vec::new();
            let mut current_conversation: Option<Conversation> = None;
            let mut current_exchange: Option<Exchange> = None;
            let mut current_messages = Vec::new();

            for row in rows {
                let (conversation, exchange, message) = row?;

                if current_conversation
                    .as_ref()
                    .map_or(true, |c| c.id != conversation.id)
                {
                    if let Some(conv) = current_conversation.take() {
                        result.push((
                            conv,
                            current_exchange.take().map(|e| {
                                (e, std::mem::take(&mut current_messages))
                            }),
                        ));
                    }
                    current_conversation = Some(conversation);
                    current_exchange = exchange;
                }

                if let Some(msg) = message {
                    current_messages.push(msg);
                }
            }

            if let Some(conv) = current_conversation.take() {
                result.push((
                    conv,
                    current_exchange.take().map(|e| (e, current_messages)),
                ));
            }

            Ok(result)
        })
    }

    fn commit_queued_operations(&mut self) -> Result<(), SqliteError> {
        let result = self.db.process_queue()?;
        eprintln!("Commit Result: {:?}", result);
        Ok(result)
    }
}
