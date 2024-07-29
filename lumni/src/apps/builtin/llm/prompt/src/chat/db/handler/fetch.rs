use super::*;

impl<'a> ConversationDbHandler<'a> {
    pub fn fetch_completion_options(
        &self,
    ) -> Result<serde_json::Value, SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
            let query =
                "SELECT completion_options FROM conversations WHERE id = ?";
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.query_row(query, params![conversation_id.0], |row| {
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
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub fn fetch_model_identifier(&self) -> Result<ModelIdentifier, SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
            let query = "
                SELECT m.identifier
                FROM conversations c
                JOIN models m ON c.model_identifier = m.identifier
                WHERE c.id = ?
            ";
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.query_row(query, params![conversation_id.0], |row| {
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
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub fn fetch_conversation_status(
            &self,
        ) -> Result<ConversationStatus, SqliteError> {
            if let Some(conversation_id) = self.conversation_id {
                let query = "SELECT status FROM conversations WHERE id = ?";
                let mut db = self.db.lock().unwrap();
                db.process_queue_with_result(|tx| {
                    tx.query_row(query, params![conversation_id.0], |row| {
                        let status: String = row.get(0)?;
                        ConversationStatus::from_str(&status).map_err(|e| {
                            SqliteError::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })
                    })
                })
            } else {
                Err(SqliteError::QueryReturnedNoRows)
            }
        }

    pub fn fetch_conversation_tags(&self) -> Result<Vec<String>, SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
            let query = "
                SELECT t.name
                FROM tags t
                JOIN conversation_tags ct ON t.id = ct.tag_id
                WHERE ct.conversation_id = ?
                ORDER BY t.name
            ";
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.prepare(query)?
                    .query_map(params![conversation_id.0], |row| row.get(0))?
                    .collect()
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub fn fetch_model_spec(&self) -> Result<ModelSpec, SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
            let query = "
                SELECT m.identifier, m.info, m.config, m.context_window_size, \
                         m.input_token_limit
                FROM conversations c
                JOIN models m ON c.model_identifier = m.identifier
                WHERE c.id = ?
            ";
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.query_row(query, params![conversation_id.0], |row| {
                    Ok(ModelSpec {
                        identifier: ModelIdentifier::new(
                            &row.get::<_, String>(0)?,
                        )
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
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub fn fetch_system_prompt(&self) -> Result<Option<String>, SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
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
                tx.query_row(query, params![conversation_id.0], |row| {
                    row.get(0)
                })
                .optional()
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub fn fetch_conversation_stats(&self) -> Result<(i64, i64), SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
            let query = "SELECT message_count, total_tokens FROM \
                         conversations WHERE id = ?";
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.query_row(query, params![conversation_id.0], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }
}

impl<'a> ConversationDbHandler<'a> {
    pub fn fetch_conversation_list(
        &self,
        limit: usize,
    ) -> Result<Vec<Conversation>, SqliteError> {
        let query = format!(
            "SELECT id, name, info, completion_options, model_identifier, 
             parent_conversation_id, fork_message_id, created_at, updated_at, 
             is_deleted, is_pinned, status, message_count, total_tokens
             FROM conversations
             WHERE is_deleted = FALSE
             ORDER BY is_pinned DESC, updated_at DESC
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
                    is_pinned: row.get::<_, i64>(10)? != 0,
                    status: ConversationStatus::from_str(
                        &row.get::<_, String>(11)?,
                    )
                    .unwrap_or(ConversationStatus::Active),
                    message_count: row.get(12)?,
                    total_tokens: row.get(13)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn fetch_last_message_id(
        &self,
    ) -> Result<Option<MessageId>, SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
            let query = "
                SELECT MAX(id) as last_message_id
                FROM messages
                WHERE conversation_id = ? AND is_deleted = FALSE
            ";

            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.query_row(query, params![conversation_id.0], |row| {
                    row.get::<_, Option<i64>>(0)
                        .map(|opt_id| opt_id.map(MessageId))
                })
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub fn fetch_messages(&self) -> Result<Vec<Message>, SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
            let query = "
                SELECT id, role, message_type, content, has_attachments, 
                       token_length, previous_message_id, created_at, vote, 
                       include_in_prompt, is_hidden, is_deleted
                FROM messages
                WHERE conversation_id = ? AND is_deleted = FALSE
                ORDER BY created_at ASC
            ";
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.prepare(query)?
                    .query_map(params![conversation_id.0], |row| {
                        Ok(Message {
                            id: MessageId(row.get(0)?),
                            conversation_id: conversation_id,
                            role: row.get(1)?,
                            message_type: row.get(2)?,
                            content: row.get(3)?,
                            has_attachments: row.get::<_, i64>(4)? != 0,
                            token_length: row.get(5)?,
                            previous_message_id: row
                                .get::<_, Option<i64>>(6)?
                                .map(MessageId),
                            created_at: row.get(7)?,
                            vote: row.get(8)?,
                            include_in_prompt: row.get::<_, i64>(9)? != 0,
                            is_hidden: row.get::<_, i64>(10)? != 0,
                            is_deleted: row.get::<_, i64>(11)? != 0,
                        })
                    })?
                    .collect()
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub fn fetch_attachments(&self) -> Result<Vec<Attachment>, SqliteError> {
        if let Some(conversation_id) = self.conversation_id {
            let query = "
                SELECT attachment_id, message_id, file_uri, file_data, \
                         file_type, metadata, created_at, is_deleted
                FROM attachments
                WHERE conversation_id = ? AND is_deleted = FALSE
                ORDER BY created_at ASC
            ";
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.prepare(query)?
                    .query_map(params![conversation_id.0], |row| {
                        Ok(Attachment {
                            attachment_id: AttachmentId(row.get(0)?),
                            message_id: MessageId(row.get(1)?),
                            conversation_id: conversation_id,
                            data: if let Some(uri) =
                                row.get::<_, Option<String>>(2)?
                            {
                                AttachmentData::Uri(uri)
                            } else {
                                AttachmentData::Data(row.get(3)?)
                            },
                            file_type: row.get(4)?,
                            metadata: row.get::<_, Option<String>>(5)?.map(
                                |s| {
                                    serde_json::from_str(&s).unwrap_or_default()
                                },
                            ),
                            created_at: row.get(6)?,
                            is_deleted: row.get::<_, i64>(7)? != 0,
                        })
                    })?
                    .collect()
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }


}
