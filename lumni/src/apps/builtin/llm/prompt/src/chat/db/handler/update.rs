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
}