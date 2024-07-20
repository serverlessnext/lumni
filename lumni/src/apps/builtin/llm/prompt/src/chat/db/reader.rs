use std::sync::{Arc, Mutex};

use rusqlite::{params, Error as SqliteError, OptionalExtension};

use super::connector::DatabaseConnector;
use super::conversation::{ConversationId, ModelIdentifier};

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

    pub fn get_conversation_stats(
        &self,
    ) -> Result<(i64, i64), SqliteError> {
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
