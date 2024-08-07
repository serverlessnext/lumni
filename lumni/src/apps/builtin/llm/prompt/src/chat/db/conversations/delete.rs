use super::*;

impl ConversationDbHandler {
    pub async fn permanent_delete_conversation(
        &mut self,
        conversation_id: Option<ConversationId>,
    ) -> Result<(), DatabaseOperationError> {
        let target_conversation_id = conversation_id.or(self.conversation_id);

        if let Some(id) = target_conversation_id {
            let mut db = self.db.lock().await;
            let result = db.process_queue_with_result(|tx| {
                // Delete all attachments for the conversation
                tx.execute(
                    "DELETE FROM attachments WHERE conversation_id = ?",
                    params![id.0],
                )?;

                // Delete all messages for the conversation
                tx.execute(
                    "DELETE FROM messages WHERE conversation_id = ?",
                    params![id.0],
                )?;

                // Delete the conversation itself
                tx.execute(
                    "DELETE FROM conversations WHERE id = ?",
                    params![id.0],
                )?;

                // Delete any conversation tags
                tx.execute(
                    "DELETE FROM conversation_tags WHERE conversation_id = ?",
                    params![id.0],
                )?;

                Ok(())
            });

            // If the deleted conversation was the one set in the handler, set it to None
            if result.is_ok() && Some(id) == self.conversation_id {
                self.conversation_id = None;
            }

            result
        } else {
            Err(DatabaseOperationError::SqliteError(
                SqliteError::QueryReturnedNoRows,
            ))
        }
    }
}
