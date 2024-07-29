use super::*;

impl<'a> ConversationDbHandler<'a> {
    pub fn update_conversation_pin_status(
        &self,
        is_pinned: bool,
        conversation_id: Option<ConversationId>,
    ) -> Result<(), SqliteError> {
        let target_conversation_id = conversation_id.or(self.conversation_id);

        if let Some(id) = target_conversation_id {
            let query = "UPDATE conversations SET is_pinned = ? WHERE id = ?";
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                tx.execute(query, params![is_pinned, id.0])?;
                Ok(())
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub fn soft_delete_conversation(
        &mut self,
        conversation_id: Option<ConversationId>,
    ) -> Result<(), SqliteError> {
        let target_conversation_id = conversation_id.or(self.conversation_id);

        if let Some(id) = target_conversation_id {
            let mut db = self.db.lock().unwrap();
            db.process_queue_with_result(|tx| {
                // Set is_deleted to TRUE for the conversation
                tx.execute(
                    "UPDATE conversations SET is_deleted = TRUE WHERE id = ?",
                    params![id.0],
                )?;

                // Set is_deleted to TRUE for all messages in the conversation
                tx.execute(
                    "UPDATE messages SET is_deleted = TRUE WHERE \
                     conversation_id = ?",
                    params![id.0],
                )?;

                // Set is_deleted to TRUE for all attachments in the conversation
                tx.execute(
                    "UPDATE attachments SET is_deleted = TRUE WHERE \
                     conversation_id = ?",
                    params![id.0],
                )?;

                Ok(())
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }
}
