use std::sync::{Arc, Mutex};

use rusqlite::{params, Error as SqliteError, OptionalExtension};

use super::connector::DatabaseConnector;
use super::conversation::{
    Attachment, AttachmentData, AttachmentId, ConversationId, Message,
    MessageId, ModelIdentifier, ModelSpec,
};

pub struct ConversationReader<'a> {
    conversation_id: ConversationId,
    db: &'a Arc<Mutex<DatabaseConnector>>,
}

impl<'a> ConversationReader<'a> {
    pub fn new(
        conversation_id: ConversationId,
        db: &'a Arc<Mutex<DatabaseConnector>>,
    ) -> Self {
        ConversationReader {
            conversation_id,
            db,
        }
    }
}

impl<'a> ConversationReader<'a> {
    pub fn get_conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    pub fn get_model_identifier(&self) -> Result<ModelIdentifier, SqliteError> {
        let query = "
            SELECT m.identifier
            FROM conversations c
            JOIN models m ON c.model_identifier = m.identifier
            WHERE c.id = ?
        ";

        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.query_row(query, params![self.conversation_id.0], |row| {
                let identifier: String = row.get(0)?;
                ModelIdentifier::new(&identifier).map_err(|e| {
                    SqliteError::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            })
        })
    }

    pub fn get_completion_options(
        &self,
    ) -> Result<serde_json::Value, SqliteError> {
        let query = "SELECT completion_options FROM conversations WHERE id = ?";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.query_row(query, params![self.conversation_id.0], |row| {
                let options: Option<String> = row.get(0)?;
                match options {
                    Some(options_str) => serde_json::from_str(&options_str)
                        .map_err(|e| {
                            SqliteError::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        }),
                    None => Ok(serde_json::json!({})),
                }
            })
        })
    }

    pub fn get_model_spec(&self) -> Result<ModelSpec, SqliteError> {
        let query = "
            SELECT m.identifier, m.info, m.config, m.context_window_size, \
                     m.input_token_limit
            FROM conversations c
            JOIN models m ON c.model_identifier = m.identifier
            WHERE c.id = ?
        ";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.query_row(query, params![self.conversation_id.0], |row| {
                Ok(ModelSpec {
                    identifier: ModelIdentifier::new(&row.get::<_, String>(0)?)
                        .unwrap(),
                    info: row
                        .get::<_, Option<String>>(1)?
                        .map(|s| serde_json::from_str(&s).unwrap()),
                    config: row
                        .get::<_, Option<String>>(2)?
                        .map(|s| serde_json::from_str(&s).unwrap()),
                    context_window_size: row.get(3)?,
                    input_token_limit: row.get(4)?,
                })
            })
        })
    }

    pub fn get_all_messages(&self) -> Result<Vec<Message>, SqliteError> {
        let query = "
            SELECT id, role, message_type, content, has_attachments, \
                     token_length, previous_message_id, created_at, is_deleted
            FROM messages
            WHERE conversation_id = ? AND is_deleted = FALSE
            ORDER BY created_at ASC
        ";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.prepare(query)?
                .query_map(params![self.conversation_id.0], |row| {
                    Ok(Message {
                        id: MessageId(row.get(0)?),
                        conversation_id: self.conversation_id,
                        role: row.get(1)?,
                        message_type: row.get(2)?,
                        content: row.get(3)?,
                        has_attachments: row.get::<_, i64>(4)? != 0,
                        token_length: row.get(5)?,
                        previous_message_id: row
                            .get::<_, Option<i64>>(6)?
                            .map(MessageId),
                        created_at: row.get(7)?,
                        is_deleted: row.get::<_, i64>(8)? != 0,
                    })
                })?
                .collect()
        })
    }

    pub fn get_all_attachments(&self) -> Result<Vec<Attachment>, SqliteError> {
        let query = "
            SELECT attachment_id, message_id, file_uri, file_data, file_type, \
                     metadata, created_at, is_deleted
            FROM attachments
            WHERE conversation_id = ? AND is_deleted = FALSE
            ORDER BY created_at ASC
        ";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.prepare(query)?
                .query_map(params![self.conversation_id.0], |row| {
                    Ok(Attachment {
                        attachment_id: AttachmentId(row.get(0)?),
                        message_id: MessageId(row.get(1)?),
                        conversation_id: self.conversation_id,
                        data: if let Some(uri) =
                            row.get::<_, Option<String>>(2)?
                        {
                            AttachmentData::Uri(uri)
                        } else {
                            AttachmentData::Data(row.get(3)?)
                        },
                        file_type: row.get(4)?,
                        metadata: row.get::<_, Option<String>>(5)?.map(|s| {
                            serde_json::from_str(&s).unwrap_or_default()
                        }),
                        created_at: row.get(6)?,
                        is_deleted: row.get::<_, i64>(7)? != 0,
                    })
                })?
                .collect()
        })
    }

    pub fn get_system_prompt(&self) -> Result<Option<String>, SqliteError> {
        let query = "
            SELECT content 
            FROM messages 
            WHERE conversation_id = ? 
              AND role = 'system' 
              AND is_deleted = FALSE 
            ORDER BY created_at ASC 
            LIMIT 1
        ";

        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.query_row(query, params![self.conversation_id.0], |row| {
                row.get(0)
            })
            .optional()
        })
    }

    pub fn get_last_message_id(
        &self,
    ) -> Result<Option<MessageId>, SqliteError> {
        let query = "
            SELECT MAX(id) as last_message_id
            FROM messages
            WHERE conversation_id = ? AND is_deleted = FALSE
        ";

        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.query_row(query, params![self.conversation_id.0], |row| {
                row.get::<_, Option<i64>>(0)
                    .map(|opt_id| opt_id.map(MessageId))
            })
        })
    }

    pub fn get_conversation_stats(&self) -> Result<(i64, i64), SqliteError> {
        let query = "SELECT message_count, total_tokens FROM conversations \
                     WHERE id = ?";
        let mut db = self.db.lock().unwrap();
        db.process_queue_with_result(|tx| {
            tx.query_row(query, params![self.conversation_id.0], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
        })
    }
}
