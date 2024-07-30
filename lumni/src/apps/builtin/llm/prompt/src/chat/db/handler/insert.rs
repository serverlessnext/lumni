use super::*;

impl<'a> ConversationDbHandler<'a> {
    pub fn new_conversation(
        &mut self,
        name: &str,
        parent_id: Option<ConversationId>,
        fork_message_id: Option<MessageId>,
        completion_options: Option<serde_json::Value>,
        model: &ModelSpec,
    ) -> Result<ConversationId, SqliteError> {
        let timestamp = Timestamp::from_system_time().unwrap().as_millis();
        let mut db = self.db.lock().unwrap();
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
                message_count: None,
                total_tokens: None,
                is_deleted: false,
                is_pinned: false,
                status: ConversationStatus::Active,
            };

            tx.execute(
                "INSERT INTO conversations (
                    name, info, model_identifier, parent_conversation_id, 
                    fork_message_id, completion_options, created_at, \
                 updated_at, 
                    message_count, total_tokens, is_deleted, is_pinned, status
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, 0, ?, ?, ?)",
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
                    conversation.is_pinned,
                    match conversation.status {
                        ConversationStatus::Active => "active",
                        ConversationStatus::Archived => "archived",
                        ConversationStatus::Deleted => "deleted",
                    },
                ],
            )?;

            let id = tx.last_insert_rowid();
            self.conversation_id = Some(ConversationId(id));
            Ok(ConversationId(id))
        })
    }

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


}