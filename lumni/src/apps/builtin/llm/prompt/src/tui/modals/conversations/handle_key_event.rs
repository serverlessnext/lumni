use super::*;

impl<'a> ConversationListModal<'a> {
    pub async fn handle_edit_mode_key_event(
        &mut self,
        key_event: &mut KeyTrack,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Enter => self.save_edited_name(handler).await?,
            KeyCode::Esc => self.cancel_edit_mode(),
            _ => {
                if let Some(edit_line) = &mut self.edit_name_line {
                    edit_line.process_edit_input(key_event)?;
                }
            }
        }
        Ok(Some(WindowEvent::Modal(ModalWindowType::ConversationList)))
    }

    pub async fn handle_normal_mode_key_event(
        &mut self,
        key_event: &mut KeyTrack,
        tab_chat: &mut ChatSession,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Up => self.move_selection_up(),
            KeyCode::Down => self.move_selection_down(),
            KeyCode::Tab => self.switch_tab(),
            KeyCode::Enter => {
                let conv = self.load_conversation(handler).await?;
                return Ok(conv);
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.handle_pin_action(handler).await?
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.handle_archive_action(handler).await?
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.handle_delete_action(handler).await?
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.handle_unarchive_undo_action(handler).await?
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.edit_conversation_name().await?
            }
            KeyCode::Esc => return Ok(Some(WindowEvent::PromptWindow(None))),
            _ => {}
        }
        Ok(Some(WindowEvent::Modal(ModalWindowType::ConversationList)))
    }

    async fn save_edited_name(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        if let Some(mut edit_line) = self.edit_name_line.take() {
            let new_name = edit_line.text_buffer().to_string();
            if let Some(index) = self.editing_index.take() {
                if let Some(conversation) = self.conversations.get_mut(index) {
                    handler.update_conversation_name(Some(conversation.id), &new_name)?;
                    conversation.name = new_name;
                }
            }
        }
        Ok(())
    }

    fn cancel_edit_mode(&mut self) {
        self.edit_name_line = None;
        self.editing_index = None;
    }

    fn move_selection_up(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
            while self.current_index > 0
                && self
                    .conversations
                    .get(self.current_index)
                    .map_or(true, |conv| conv.status != self.current_tab)
            {
                self.current_index -= 1;
            }
        }
    }

    fn move_selection_down(&mut self) {
        let filtered_count = self
            .conversations
            .iter()
            .filter(|conv| conv.status == self.current_tab)
            .count();
        if filtered_count > 0
            && self.current_index < self.conversations.len() - 1
        {
            self.current_index += 1;
            while self.current_index < self.conversations.len()
                && self.conversations[self.current_index].status
                    != self.current_tab
            {
                self.current_index += 1;
            }
            if self.current_index >= self.conversations.len() {
                self.current_index = filtered_count - 1;
            }
        }
    }

    fn switch_tab(&mut self) {
        self.current_tab = match self.current_tab {
            ConversationStatus::Active => ConversationStatus::Archived,
            ConversationStatus::Archived => ConversationStatus::Deleted,
            ConversationStatus::Deleted => ConversationStatus::Active,
        };
        self.current_index = 0;
    }

    async fn handle_pin_action(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        if self.current_tab == ConversationStatus::Active {
            self.toggle_pin_status(handler).await?;
        }
        Ok(())
    }

    async fn handle_archive_action(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        if self.current_tab == ConversationStatus::Active {
            self.archive_conversation(handler).await?;
        }
        Ok(())
    }

    async fn handle_delete_action(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        match self.current_tab {
            ConversationStatus::Active | ConversationStatus::Archived => {
                self.soft_delete_conversation(handler).await?
            }
            ConversationStatus::Deleted => {
                self.permanent_delete_conversation(handler).await?
            }
        }
        Ok(())
    }

    async fn handle_unarchive_undo_action(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        match self.current_tab {
            ConversationStatus::Archived => {
                self.unarchive_conversation(handler).await?
            }
            ConversationStatus::Deleted => {
                self.undo_delete_conversation(handler).await?
            }
            _ => {}
        }
        Ok(())
    }
}
