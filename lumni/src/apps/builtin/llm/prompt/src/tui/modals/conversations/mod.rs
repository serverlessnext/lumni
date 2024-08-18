mod handle_key_event;
mod render_on_frame;

use std::collections::HashMap;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use lumni::Timestamp;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph,
    Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs,
};
use ratatui::Frame;

use super::{
    ApplicationError, CommandLine, Conversation, ConversationDbHandler,
    ConversationStatus, KeyTrack, ModalAction, ModalWindowTrait,
    ModalWindowType, PromptInstruction, TextWindowTrait, ThreadedChatSession,
    UserEvent, WindowEvent,
};
use crate::apps::builtin::llm::prompt::src::chat::db::ConversationId;
pub use crate::external as lumni;

const MAX_WIDTH: u16 = 40;
const MAX_HEIGHT: u16 = 60;

pub struct ConversationListModal<'a> {
    conversations: Vec<Conversation>,
    current_tab: ConversationStatus,
    tab_indices: HashMap<ConversationStatus, usize>,
    edit_name_line: Option<CommandLine<'a>>,
    editing_index: Option<usize>,
    last_selected_conversation_id: Option<ConversationId>,
}

impl<'a> ConversationListModal<'a> {
    pub async fn new(
        handler: ConversationDbHandler,
    ) -> Result<Self, ApplicationError> {
        let conversations = handler.fetch_conversation_list(100).await?;

        let mut tab_indices = HashMap::new();
        tab_indices.insert(ConversationStatus::Active, 0);
        tab_indices.insert(ConversationStatus::Archived, 0);
        tab_indices.insert(ConversationStatus::Deleted, 0);

        Ok(Self {
            conversations,
            current_tab: ConversationStatus::Active,
            tab_indices,
            edit_name_line: None,
            editing_index: None,
            last_selected_conversation_id: None,
        })
    }

    fn format_timestamp(timestamp: i64) -> String {
        Timestamp::new(timestamp)
            .format("[year]-[month]-[day] [hour]:[minute]:[second]")
            .unwrap_or_else(|_| "Invalid timestamp".to_string())
    }

    fn truncate_text(text: &str, max_width: usize) -> String {
        if text.len() <= max_width {
            text.to_string()
        } else {
            format!("{}...", &text[..max_width - 3])
        }
    }

    fn max_area_size(&self) -> (u16, u16) {
        (MAX_WIDTH, MAX_HEIGHT)
    }

    async fn load_conversation(
        &self,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<Option<PromptInstruction>, ApplicationError> {
        if let Some(conversation) = self.get_current_conversation() {
            db_handler.set_conversation_id(conversation.id);
            let prompt_instruction =
                PromptInstruction::from_reader(db_handler).await?;
            Ok(Some(prompt_instruction))
        } else {
            Ok(None)
        }
    }

    async fn toggle_pin_status(
        &mut self,
        handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            let new_pin_status = !conversation.is_pinned;
            handler
                .update_conversation_pin_status(
                    Some(conversation.id),
                    new_pin_status,
                )
                .await?;
            conversation.is_pinned = new_pin_status;

            // Sort only conversations in the current tab
            let current_tab = self.current_tab;
            self.conversations.sort_by(|a, b| {
                if a.status == current_tab && b.status == current_tab {
                    b.is_pinned
                        .cmp(&a.is_pinned)
                        .then(b.updated_at.cmp(&a.updated_at))
                } else {
                    std::cmp::Ordering::Equal
                }
            });

            self.adjust_indices();
        }
        Ok(())
    }

    fn adjust_indices(&mut self) {
        for status in &[
            ConversationStatus::Active,
            ConversationStatus::Archived,
            ConversationStatus::Deleted,
        ] {
            let count = self.conversations_in_tab(*status).count();
            let current_index = self.tab_indices.entry(*status).or_insert(0);
            if count == 0 {
                *current_index = 0;
            } else {
                *current_index = (*current_index).min(count - 1);
            }
        }
    }

    fn conversations_in_tab(
        &self,
        status: ConversationStatus,
    ) -> impl Iterator<Item = &Conversation> {
        self.conversations
            .iter()
            .filter(move |conv| conv.status == status)
    }

    async fn archive_conversation(
        &mut self,
        handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            handler.archive_conversation(Some(conversation.id)).await?;
            conversation.status = ConversationStatus::Archived;
        }
        self.adjust_indices();
        Ok(())
    }

    async fn unarchive_conversation(
        &mut self,
        handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            handler
                .unarchive_conversation(Some(conversation.id))
                .await?;
            conversation.status = ConversationStatus::Active;
        }
        self.adjust_indices();
        Ok(())
    }

    async fn soft_delete_conversation(
        &mut self,
        handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            handler
                .soft_delete_conversation(Some(conversation.id))
                .await?;
            conversation.status = ConversationStatus::Deleted;
        }
        self.adjust_indices();
        Ok(())
    }

    async fn undo_delete_conversation(
        &mut self,
        handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            handler
                .undo_delete_conversation(Some(conversation.id))
                .await?;
            conversation.status = ConversationStatus::Active;
        }
        self.adjust_indices();
        Ok(())
    }

    async fn permanent_delete_conversation(
        &mut self,
        handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation() {
            let id = conversation.id;
            handler.permanent_delete_conversation(Some(id)).await?;
            self.conversations.retain(|c| c.id != id);

            self.adjust_indices();

            if self.conversations_in_current_tab().count() == 0 {
                // Switch to Active tab if current tab is empty
                self.current_tab = ConversationStatus::Active;
                self.adjust_indices();
            }
        }
        Ok(())
    }

    fn get_current_conversation(&self) -> Option<&Conversation> {
        let index = *self.tab_indices.get(&self.current_tab).unwrap_or(&0);
        self.conversations_in_current_tab().nth(index)
    }

    fn get_current_conversation_mut(&mut self) -> Option<&mut Conversation> {
        let index = *self.tab_indices.get(&self.current_tab).unwrap_or(&0);
        self.conversations
            .iter_mut()
            .filter(|conv| conv.status == self.current_tab)
            .nth(index)
    }

    async fn edit_conversation_name(&mut self) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation() {
            let mut command_line = CommandLine::new();
            command_line.text_set(&conversation.name, None)?;
            command_line.set_status_insert();
            self.edit_name_line = Some(command_line);
            self.editing_index =
                Some(*self.tab_indices.get(&self.current_tab).unwrap_or(&0));
        }
        Ok(())
    }

    async fn load_and_set_conversation(
        &mut self,
        tab_chat: &mut ThreadedChatSession,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        let prompt_instruction = self.load_conversation(db_handler).await?;
        match prompt_instruction {
            Some(prompt_instruction) => {
                tab_chat.load_instruction(prompt_instruction).await?;
            }
            None => {}
        }
        Ok(())
    }

    async fn undo_delete_and_load_conversation(
        &mut self,
        tab_chat: &mut ThreadedChatSession,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            let conversation_id = conversation.id;
            db_handler
                .undo_delete_conversation(Some(conversation_id))
                .await?;
            conversation.status = ConversationStatus::Active;
            self.adjust_indices();

            // Switch to Active tab
            self.current_tab = ConversationStatus::Active;
            self.adjust_indices();

            // Now load the conversation
            db_handler.set_conversation_id(conversation_id);
            let prompt_instruction =
                PromptInstruction::from_reader(db_handler).await?;
            tab_chat.load_instruction(prompt_instruction).await?;
            self.last_selected_conversation_id = None;
        }
        Ok(())
    }
}

#[async_trait]
impl<'a> ModalWindowTrait for ConversationListModal<'a> {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::ConversationList
    }

    fn render_on_frame(&mut self, frame: &mut Frame, mut area: Rect) {
        let (max_width, max_height) = self.max_area_size();
        area = self.adjust_area(area, max_width, max_height);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Height for Details box
                Constraint::Length(3), // Height for tabs
                Constraint::Min(1),    // Remaining space for Conversations list
                Constraint::Length(2), // Height for key bindings info
            ])
            .split(area);

        frame.render_widget(Clear, area);

        self.render_details(frame, chunks[0]);
        self.render_tabs(frame, chunks[1]);
        self.render_conversations_list(frame, chunks[2]);
        self.render_key_bindings(frame, chunks[3]);
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        tab_chat: &'b mut ThreadedChatSession,
        handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError> {
        log::debug!(
            "Key: {:?}, Modifiers: {:?}",
            key_event.current_key().code,
            key_event.current_key().modifiers
        );
        match self.edit_name_line {
            Some(_) => {
                self.handle_edit_mode_key_event(key_event, handler).await
            }
            None => {
                self.handle_normal_mode_key_event(key_event, tab_chat, handler)
                    .await
            }
        }
    }
}
