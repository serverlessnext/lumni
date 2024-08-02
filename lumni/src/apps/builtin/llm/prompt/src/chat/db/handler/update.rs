use super::*;

impl ConversationDbHandler {
    pub async fn update_conversation_name(
        &self,
        conversation_id: Option<ConversationId>,
        new_name: &str,
    ) -> Result<(), SqliteError> {
        let target_conversation_id = conversation_id.or(self.conversation_id);
        let system_time = Timestamp::from_system_time().unwrap().as_millis();

        if let Some(id) = target_conversation_id {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                tx.execute(
                    "UPDATE conversations SET name = ?, updated_at = ? WHERE \
                     id = ?",
                    params![new_name, system_time, id.0],
                )?;
                Ok(())
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub async fn update_conversation_pin_status(
        &self,
        conversation_id: Option<ConversationId>,
        is_pinned: bool,
    ) -> Result<(), SqliteError> {
        let target_conversation_id = conversation_id.or(self.conversation_id);

        if let Some(id) = target_conversation_id {
            let query = "UPDATE conversations SET is_pinned = ? WHERE id = ?";
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                tx.execute(query, params![is_pinned, id.0])?;
                Ok(())
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    // used for archive and unarchive
    async fn update_conversation_status(
        &self,
        conversation_id: Option<ConversationId>,
        new_status: ConversationStatus,
    ) -> Result<(), SqliteError> {
        let target_conversation_id = conversation_id.or(self.conversation_id);

        if let Some(id) = target_conversation_id {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                tx.execute(
                    "UPDATE conversations SET status = ? WHERE id = ?",
                    params![new_status.to_string(), id.0],
                )?;
                Ok(())
            })
        } else {
            Err(SqliteError::QueryReturnedNoRows)
        }
    }

    pub async fn archive_conversation(
        &mut self,
        conversation_id: Option<ConversationId>,
    ) -> Result<(), SqliteError> {
        self.update_conversation_status(
            conversation_id,
            ConversationStatus::Archived,
        )
        .await
    }

    pub async fn unarchive_conversation(
        &mut self,
        conversation_id: Option<ConversationId>,
    ) -> Result<(), SqliteError> {
        self.update_conversation_status(
            conversation_id,
            ConversationStatus::Active,
        )
        .await
    }

    pub async fn soft_delete_conversation(
        &mut self,
        conversation_id: Option<ConversationId>,
    ) -> Result<(), SqliteError> {
        // NOTE: cant use update_conversation_status because we need to update messages and attachments as well within a single transaction
        let target_conversation_id = conversation_id.or(self.conversation_id);

        if let Some(id) = target_conversation_id {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                // Update the conversation status
                tx.execute(
                    "UPDATE conversations SET status = ? WHERE id = ?",
                    params![ConversationStatus::Deleted.to_string(), id.0],
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

    pub async fn undo_delete_conversation(
        &mut self,
        conversation_id: Option<ConversationId>,
    ) -> Result<(), SqliteError> {
        // NOTE: cant use update_conversation_status because we need to update messages and attachments as well within a single transaction
        let target_conversation_id = conversation_id.or(self.conversation_id);

        if let Some(id) = target_conversation_id {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                // Update the conversation status
                tx.execute(
                    "UPDATE conversations SET status = ? WHERE id = ?",
                    params![ConversationStatus::Active.to_string(), id.0],
                )?;

                // Set is_deleted to FALSE for all messages in the conversation
                tx.execute(
                    "UPDATE messages SET is_deleted = FALSE WHERE \
                     conversation_id = ?",
                    params![id.0],
                )?;

                // Set is_deleted to FALSE for all attachments in the conversation
                tx.execute(
                    "UPDATE attachments SET is_deleted = FALSE WHERE \
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
