use super::*;

impl<'a> ConversationListModal<'a> {
    async fn reload_conversation(
        &mut self,
        tab_chat: &mut ChatSession,
        db_handler: &mut ConversationDbHandler<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match self.current_tab {
            ConversationStatus::Deleted => {
                self.undo_delete_and_load_conversation(tab_chat, db_handler)
                    .await?
            }
            _ => self.load_and_set_conversation(tab_chat, db_handler).await?,
        }
        Ok(Some(WindowEvent::Modal(ModalWindowType::ConversationList(
            Some(ConversationEvent::ReloadConversation),
        ))))
    }

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
        Ok(Some(WindowEvent::Modal(ModalWindowType::ConversationList(
            None,
        ))))
    }

    pub async fn handle_normal_mode_key_event(
        &mut self,
        key_event: &mut KeyTrack,
        tab_chat: &mut ChatSession,
        db_handler: &mut ConversationDbHandler<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Up => {
                self.move_selection_up();
                self.last_selected_conversation_id = None;
                return self.reload_conversation(tab_chat, db_handler).await;
            }
            KeyCode::Down => {
                self.move_selection_down();
                self.last_selected_conversation_id = None;
                return self.reload_conversation(tab_chat, db_handler).await;
            }
            KeyCode::Enter => {
                return Ok(Some(WindowEvent::PromptWindow(None)));
            }
            KeyCode::Tab => {
                self.switch_tab();
                self.last_selected_conversation_id = None;
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.handle_pin_action(db_handler).await?
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.handle_archive_action(db_handler).await?
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.handle_delete_action(db_handler).await?
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.handle_unarchive_undo_action(db_handler).await?
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.edit_conversation_name().await?
            }
            KeyCode::Esc => return Ok(Some(WindowEvent::PromptWindow(None))),
            _ => {}
        }
        // stay in the Modal window
        Ok(Some(WindowEvent::Modal(ModalWindowType::ConversationList(
            None,
        ))))
    }

    async fn save_edited_name(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        if let Some(mut edit_line) = self.edit_name_line.take() {
            let new_name = edit_line.text_buffer().to_string();
            if let Some(conversation) = self.get_current_conversation_mut() {
                handler.update_conversation_name(
                    Some(conversation.id),
                    &new_name,
                )?;
                conversation.name = new_name;
            }
        }
        self.editing_index = None;
        Ok(())
    }
    fn cancel_edit_mode(&mut self) {
        self.edit_name_line = None;
        self.editing_index = None;
    }

    fn move_selection_up(&mut self) {
        let filtered_count = self.conversations_in_current_tab().count();
        if filtered_count > 0 {
            let current_index =
                self.tab_indices.get_mut(&self.current_tab).unwrap();
            if *current_index > 0 {
                *current_index -= 1;
            } else {
                *current_index = filtered_count - 1;
            }
        }
    }

    fn move_selection_down(&mut self) {
        let filtered_count = self.conversations_in_current_tab().count();
        if filtered_count > 0 {
            let current_index =
                self.tab_indices.get_mut(&self.current_tab).unwrap();
            *current_index = (*current_index + 1) % filtered_count;
        }
    }

    fn switch_tab(&mut self) {
        self.current_tab = match self.current_tab {
            ConversationStatus::Active => ConversationStatus::Archived,
            ConversationStatus::Archived => ConversationStatus::Deleted,
            ConversationStatus::Deleted => ConversationStatus::Active,
        };
        self.adjust_indices();
    }

    pub fn conversations_in_current_tab(
        &self,
    ) -> impl Iterator<Item = &Conversation> {
        self.conversations
            .iter()
            .filter(|conv| conv.status == self.current_tab)
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
