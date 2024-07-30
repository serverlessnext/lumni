mod handle_key_event;
mod render_on_frame;

use async_trait::async_trait;
use crossterm::event::KeyCode;
pub use lumni::Timestamp;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph,
    Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs,
};
use ratatui::Frame;

use super::{
    ApplicationError, ChatSession, CommandLine, Conversation,
    ConversationDbHandler, ConversationEvent, ConversationStatus, KeyTrack,
    ModalWindowTrait, ModalWindowType, PromptInstruction,
    TextWindowTrait, WindowEvent,
};
pub use crate::external as lumni;

const MAX_WIDTH: u16 = 40;
const MAX_HEIGHT: u16 = 60;

pub struct ConversationListModal<'a> {
    conversations: Vec<Conversation>,
    current_index: usize,
    current_tab: ConversationStatus,
    edit_name_line: Option<CommandLine<'a>>,
    editing_index: Option<usize>,
}

impl<'a> ConversationListModal<'a> {
    pub fn new(
        handler: &ConversationDbHandler<'_>,
    ) -> Result<Self, ApplicationError> {
        let conversations = handler.fetch_conversation_list(100)?;
        Ok(Self {
            conversations,
            current_index: 0,
            current_tab: ConversationStatus::Active,
            edit_name_line: None,
            editing_index: None,
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
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        if let Some(conversation) = self.get_current_conversation() {
            handler.set_conversation_id(conversation.id);
            match PromptInstruction::from_reader(handler) {
                Ok(prompt_instruction) => {
                    Ok(Some(WindowEvent::PromptWindow(
                        Some(ConversationEvent::ContinueConversation(
                            prompt_instruction,
                    )))))
                }
                Err(e) => Err(e),
            }
        } else {
            Ok(None)
        }
    }

    async fn toggle_pin_status(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        let conversation_id = self
            .get_current_conversation()
            .map(|conv| (conv.id, conv.is_pinned));

        if let Some((id, is_pinned)) = conversation_id {
            let new_pin_status = !is_pinned;
            handler.update_conversation_pin_status(Some(id), new_pin_status)?;

            // Update the local list
            if let Some(conv) =
                self.conversations.iter_mut().find(|c| c.id == id)
            {
                conv.is_pinned = new_pin_status;
            }

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

            // Update the current_index to match the moved conversation
            self.current_index = self
                .conversations
                .iter()
                .position(|c| c.id == id)
                .unwrap_or(0);
        }
        Ok(())
    }

    async fn archive_conversation(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            handler.archive_conversation(Some(conversation.id))?;
            conversation.status = ConversationStatus::Archived;
        }
        Ok(())
    }

    async fn unarchive_conversation(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            handler.unarchive_conversation(Some(conversation.id))?;
            conversation.status = ConversationStatus::Active;
        }
        Ok(())
    }

    async fn soft_delete_conversation(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            handler.soft_delete_conversation(Some(conversation.id))?;
            conversation.status = ConversationStatus::Deleted;
        }
        Ok(())
    }

    async fn undo_delete_conversation(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation_mut() {
            handler.undo_delete_conversation(Some(conversation.id))?;
            conversation.status = ConversationStatus::Active;
        }
        Ok(())
    }

    async fn permanent_delete_conversation(
        &mut self,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        let conversation_id =
            self.get_current_conversation().map(|conv| conv.id);

        if let Some(id) = conversation_id {
            handler.permanent_delete_conversation(Some(id))?;
            self.conversations.retain(|c| c.id != id);

            let filtered_count = self
                .conversations
                .iter()
                .filter(|conv| conv.status == self.current_tab)
                .count();

            if filtered_count == 0 {
                // Switch to Active tab if current tab is empty
                self.current_tab = ConversationStatus::Active;
            }

            self.current_index =
                self.current_index.min(filtered_count.saturating_sub(1));
        }
        Ok(())
    }

    fn get_current_conversation(&self) -> Option<&Conversation> {
        self.conversations
            .iter()
            .filter(|conv| conv.status == self.current_tab)
            .nth(self.current_index)
    }

    fn get_current_conversation_mut(&mut self) -> Option<&mut Conversation> {
        self.conversations
            .iter_mut()
            .filter(|conv| conv.status == self.current_tab)
            .nth(self.current_index)
    }

    async fn edit_conversation_name(
        &mut self,
    ) -> Result<(), ApplicationError> {
        if let Some(conversation) = self.get_current_conversation() {
            let mut command_line = CommandLine::new();
            command_line.text_set(&conversation.name, None)?;
            command_line.set_status_insert();
            self.edit_name_line = Some(command_line);
            self.editing_index = Some(self.current_index);
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
        tab_chat: &'b mut ChatSession,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
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
