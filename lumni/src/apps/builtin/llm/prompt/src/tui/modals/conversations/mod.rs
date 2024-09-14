use std::collections::HashMap;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyModifiers};
use lumni::Timestamp;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::widgets::{ListWidget, ListWidgetState};
use super::{
    ApplicationError, ChatSessionManager, Conversation, ConversationDbHandler,
    ConversationEvent, ConversationId, ConversationStatus, KeyTrack,
    ModalEvent, ModalWindowTrait, ModalWindowType, UserEvent, WindowMode,
};
pub use crate::external as lumni;

const MAX_WIDTH: u16 = 34;

pub struct ConversationListModal {
    conversations: Vec<Conversation>,
    current_tab: ConversationStatus,
    tab_indices: HashMap<ConversationStatus, usize>,
    last_selected_conversation_id: Option<ConversationId>,
    list_widget: ListWidget,
    list_widget_state: ListWidgetState,
}

impl ConversationListModal {
    pub async fn new(
        handler: ConversationDbHandler,
    ) -> Result<Self, ApplicationError> {
        let conversations = handler.fetch_conversation_list(100).await?;

        let mut tab_indices = HashMap::new();
        tab_indices.insert(ConversationStatus::Active, 0);
        tab_indices.insert(ConversationStatus::Archived, 0);
        tab_indices.insert(ConversationStatus::Deleted, 0);

        let list_widget = ListWidget::new(Vec::new())
            .normal_style(
                Style::default().bg(Color::Rgb(24, 32, 40)).fg(Color::Gray),
            )
            .selected_style(
                Style::default().bg(Color::Rgb(32, 40, 48)).fg(Color::Cyan),
            )
            .highlight_symbol("► ".to_string())
            .show_borders(false);

        Ok(Self {
            conversations,
            current_tab: ConversationStatus::Active,
            tab_indices,
            last_selected_conversation_id: None,
            list_widget,
            list_widget_state: ListWidgetState::default(),
        })
    }

    async fn reload_conversation(
        &mut self,
        chat_manager: &mut ChatSessionManager,
    ) -> Result<WindowMode, ApplicationError> {
        if let Some(conversation) = self.get_current_conversation() {
            chat_manager.load_conversation(conversation.clone()).await?;
        }
        Ok(WindowMode::Modal(ModalEvent::Event(
            UserEvent::ReloadConversation,
        )))
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: &mut KeyTrack,
        chat_manager: &mut ChatSessionManager,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError> {
        let current_key = key_event.current_key();
        if current_key.modifiers == KeyModifiers::SHIFT {
            match current_key.code {
                KeyCode::BackTab | KeyCode::Left => {
                    return Ok(WindowMode::Conversation(Some(
                        ConversationEvent::PromptRead,
                    )));
                }
                KeyCode::Up => {}
                _ => {}
            }
        }

        match current_key.code {
            KeyCode::Up => {
                self.move_selection_up();
                self.last_selected_conversation_id = None;
                return self.reload_conversation(chat_manager).await;
            }
            KeyCode::Down => {
                self.move_selection_down();
                self.last_selected_conversation_id = None;
                return self.reload_conversation(chat_manager).await;
            }
            KeyCode::Enter => {
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::PromptRead,
                )));
            }
            KeyCode::Char('q') | KeyCode::BackTab => {
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
            KeyCode::Esc => {
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::PromptRead,
                )))
            }
            _ => {}
        }
        // stay in the Modal window, waiting for next key event
        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
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
        handler: &ConversationDbHandler,
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

    pub fn adjust_area(&self, mut area: Rect, max_width: u16) -> Rect {
        area.x = area.width.saturating_sub(max_width);
        area.y = area.y + 0;
        area.width = max_width;
        area.height = area.height.saturating_sub(0);
        area
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let titles = vec!["Active", "Archived", "Deleted"];

        // Render the top border
        frame.render_widget(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
            area,
        );

        // Calculate total width of all titles and spaces between them
        let total_width: u16 =
            titles.iter().map(|t| t.len() as u16).sum::<u16>()
                + (titles.len() as u16 - 1) * 3;
        let start_x =
            area.left() + (area.width.saturating_sub(total_width)) / 2;

        let mut x = start_x;
        for (i, title) in titles.iter().enumerate() {
            let style = if i == self.current_tab as usize {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            // Render the title
            frame.render_widget(
                Paragraph::new(*title).style(style),
                Rect::new(x, area.top() + 1, title.len() as u16, 1),
            );

            x += title.len() as u16;

            // Add separator between tabs, but not after the last one
            if i < titles.len() - 1 {
                x += 1;
                frame.render_widget(
                    Paragraph::new("│")
                        .style(Style::default().fg(Color::DarkGray)),
                    Rect::new(x, area.top() + 1, 1, 1),
                );
                x += 2;
            }
        }
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

    pub fn render_key_bindings(&self, frame: &mut Frame, area: Rect) {
        let key_info = match self.current_tab {
            ConversationStatus::Active => {
                "↑↓: Navigate | Enter: Select | P: Toggle Pin | A: Archive | \
                 D: Delete | E: Edit Name | Tab: Switch Tab | Esc: Close"
            }
            ConversationStatus::Archived => {
                "↑↓: Navigate | Enter: Select | U: Unarchive | E: Edit Name | \
                 Tab: Switch Tab | Esc: Close"
            }
            ConversationStatus::Deleted => {
                "↑↓: Navigate | Enter: Undo & Select | U: Undo Delete | D: \
                 Permanent Delete | E: Edit Name | Tab: Switch Tab | Esc: Close"
            }
        };
        let key_info =
            Paragraph::new(key_info).style(Style::default().fg(Color::Cyan));
        frame.render_widget(key_info, area);
    }

    pub fn render_on_frame(&mut self, frame: &mut Frame, mut area: Rect) {
        area = self.adjust_area(area, MAX_WIDTH);
        frame.render_widget(Clear, area);

        // Create a block for the entire modal with a left border
        let modal_block = Block::default()
            .title("💬 Conversations")
            .title_alignment(Alignment::Center)
            .title_style(Style::default().fg(Color::Cyan))
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(Color::Rgb(16, 24, 32)));

        // Create an inner area for the content
        let inner_area = modal_block.inner(area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Space for the separator
                Constraint::Min(1),    // Space for Conversations list
                Constraint::Length(3), // Height for tabs at the bottom
            ])
            .split(inner_area);

        let separator = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(separator, chunks[0]);

        frame.render_widget(modal_block, area);
        self.render_conversations_list(frame, chunks[1]);
        self.render_tabs(frame, chunks[2]);
    }
}

#[async_trait]
impl ModalWindowTrait for ConversationListModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::ConversationList
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        self.render_on_frame(frame, area);
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        chat_manager: &mut ChatSessionManager,
        handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError> {
        self.handle_key_event(key_event, chat_manager, handler)
            .await
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
