use std::collections::HashMap;

use crossterm::event::KeyCode;
use lumni::api::error::ApplicationError;
use lumni::Timestamp;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::Frame;

use super::widgets::{ListWidget, ListWidgetState};
use super::{
    Conversation, ConversationDbHandler, ConversationEvent, ConversationId,
    ConversationSelectEvent, ConversationStatus, KeyTrack, PromptInstruction,
    ThreadedChatSession, WindowMode,
};
pub use crate::external as lumni;

#[derive(Debug)]
pub struct Conversations {
    conversations: Vec<Conversation>,
    current_tab: ConversationStatus,
    tab_indices: HashMap<ConversationStatus, usize>,
    list_widget: ListWidget,
    list_widget_state: ListWidgetState,
    last_selected_conversation_id: Option<ConversationId>,
}

impl Conversations {
    pub fn new(conversations: Vec<Conversation>) -> Self {
        let mut tab_indices = HashMap::new();
        tab_indices.insert(ConversationStatus::Active, 0);
        tab_indices.insert(ConversationStatus::Archived, 0);
        tab_indices.insert(ConversationStatus::Deleted, 0);

        let list_widget = ListWidget::new(Vec::new())
            .title("Conversations")
            .normal_style(
                Style::default().bg(Color::Rgb(24, 32, 40)).fg(Color::Gray),
            )
            .selected_style(
                Style::default().bg(Color::Rgb(32, 40, 48)).fg(Color::Cyan),
            )
            .highlight_symbol("► ".to_string())
            .show_borders(false);

        Self {
            conversations,
            current_tab: ConversationStatus::Active,
            tab_indices,
            list_widget,
            list_widget_state: ListWidgetState::default(),
            last_selected_conversation_id: None,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Space for Conversations list
                Constraint::Length(3), // Height for tabs at the bottom
            ])
            .split(area);

        self.render_conversations_list(frame, chunks[0]);
        self.render_tabs(frame, chunks[1]);
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tabs = vec!["Active", "Archived", "Deleted"];
        let tabs = Tabs::new(tabs)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .select(match self.current_tab {
                ConversationStatus::Active => 0,
                ConversationStatus::Archived => 1,
                ConversationStatus::Deleted => 2,
            })
            .style(Style::default().fg(Color::Gray))
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(tabs, area);
    }

    fn render_conversations_list(&mut self, frame: &mut Frame, area: Rect) {
        let conversations = self
            .conversations_in_current_tab()
            .enumerate()
            .map(|(index, conversation)| {
                self.create_conversation_list_item(conversation, index)
            })
            .collect::<Vec<Text>>();

        self.list_widget.items = conversations;

        frame.render_stateful_widget(
            &self.list_widget,
            area,
            &mut self.list_widget_state,
        );
    }

    fn create_conversation_list_item(
        &self,
        conversation: &Conversation,
        index: usize,
    ) -> Text<'static> {
        let is_selected = index == self.list_widget_state.selected_index;
        let base_style = if is_selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        let pin_indicator = if conversation.is_pinned {
            "📌 "
        } else {
            "  "
        };
        let name = truncate_text(&conversation.name, 30);
        let updated = format_timestamp(conversation.updated_at);
        let tokens = conversation.total_tokens.unwrap_or(0);
        let messages = conversation.message_count.unwrap_or(0);

        Text::from(vec![
            Line::from(vec![
                Span::styled(pin_indicator, base_style),
                Span::styled(name, base_style.add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
                Span::styled(updated, base_style),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("Tokens: {} ", tokens),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("Messages: {}", messages),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
        ])
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

    fn move_selection_up(&mut self) {
        self.list_widget
            .move_selection(&mut self.list_widget_state, -1);
        *self.tab_indices.get_mut(&self.current_tab).unwrap() =
            self.list_widget_state.selected_index;
    }

    fn move_selection_down(&mut self) {
        self.list_widget
            .move_selection(&mut self.list_widget_state, 1);
        *self.tab_indices.get_mut(&self.current_tab).unwrap() =
            self.list_widget_state.selected_index;
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
        handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if self.current_tab == ConversationStatus::Active {
            self.toggle_pin_status(handler).await?;
        }
        Ok(())
    }

    async fn handle_archive_action(
        &mut self,
        handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if self.current_tab == ConversationStatus::Active {
            self.archive_conversation(handler).await?;
        }
        Ok(())
    }

    async fn handle_delete_action(
        &mut self,
        handler: &mut ConversationDbHandler,
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
        handler: &mut ConversationDbHandler,
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

    async fn reload_conversation(
        &mut self,
        tab_chat: &mut ThreadedChatSession,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError> {
        match self.current_tab {
            ConversationStatus::Deleted => {
                self.undo_delete_and_load_conversation(tab_chat, db_handler)
                    .await?
            }
            _ => self.load_and_set_conversation(tab_chat, db_handler).await?,
        }

        Ok(WindowMode::Conversation(Some(ConversationEvent::Select(
            Some(ConversationSelectEvent::ReloadConversation),
        ))))
    }
}

impl Conversations {
    pub async fn handle_key_event(
        &mut self,
        key_event: &mut KeyTrack,
        tab_chat: Option<&mut ThreadedChatSession>,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Up => {
                self.move_selection_up();
                self.last_selected_conversation_id = None;
                if let Some(tab_chat) = tab_chat {
                    return self
                        .reload_conversation(tab_chat, db_handler)
                        .await;
                }
                log::warn!("ThreadedChatSession is not available");
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::Select(None),
                )));
            }
            KeyCode::Down => {
                self.move_selection_down();
                self.last_selected_conversation_id = None;
                if let Some(tab_chat) = tab_chat {
                    return self
                        .reload_conversation(tab_chat, db_handler)
                        .await;
                }
                log::warn!("ThreadedChatSession is not available");
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::Select(None),
                )));
            }
            KeyCode::Enter => {
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::PromptRead,
                )));
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
            KeyCode::Char('i') | KeyCode::Char('I') => {
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::PromptInsert,
                )));
            }
            _ => {}
        }
        // stay in the select window, waiting for next key event
        Ok(WindowMode::Conversation(Some(ConversationEvent::Select(
            None,
        ))))
    }
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
