use std::collections::HashMap;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use lumni::Timestamp;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Tabs};
use ratatui::Frame;

use super::widgets::{ListWidget, ListWidgetState};
use super::{
    ApplicationError, Conversation, ConversationDbHandler, ConversationEvent,
    ConversationId, ConversationStatus, KeyTrack, ModalEvent, ModalWindowTrait,
    ModalWindowType, PromptInstruction, ThreadedChatSession, UserEvent,
    WindowMode,
};
pub use crate::external as lumni;

const MAX_WIDTH: u16 = 36;
const MAX_HEIGHT: u16 = 60;

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

        let list_widget =
            ListWidget::new(Vec::new(), "Conversations".to_string())
                .normal_style(Style::default().bg(Color::Black).fg(Color::Cyan))
                .selected_style(
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                )
                .highlight_symbol(">> ".to_string());

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
        Ok(WindowMode::Modal(ModalEvent::Event(
            UserEvent::ReloadConversation,
        )))
    }

    pub async fn handle_normal_mode_key_event(
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
                return Ok(WindowMode::Modal(ModalEvent::UpdateUI));
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
                return Ok(WindowMode::Modal(ModalEvent::UpdateUI));
            }
            KeyCode::Enter => {
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::Prompt,
                )));
            }
            KeyCode::Char('q') => {
                return Ok(WindowMode::Conversation(Some(
                    ConversationEvent::Prompt,
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
                    ConversationEvent::Prompt,
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

    fn update_list_widget(&mut self) {
        let items: Vec<Text<'static>> = self
            .conversations_in_current_tab()
            .enumerate()
            .map(|(index, conversation)| {
                self.create_conversation_list_item(conversation, index)
            })
            .collect();

        self.list_widget = ListWidget::new(items, "Conversations".to_string())
            .normal_style(Style::default().bg(Color::Black).fg(Color::Cyan))
            .selected_style(
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )
            .highlight_symbol(">> ".to_string());
    }

    fn create_conversation_list_item(
        &self,
        conversation: &Conversation,
        index: usize,
    ) -> Text<'static> {
        let is_selected = index == self.list_widget_state.selected_index;
        let base_style = if is_selected {
            Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
        } else {
            Style::default().bg(Color::Black).fg(Color::Cyan)
        };

        let pin_indicator = if conversation.is_pinned {
            "📌 "
        } else {
            "  "
        };
        let name =
            Self::truncate_text(&conversation.name, self.max_width() - 5);
        let updated = Self::format_timestamp(conversation.updated_at);
        let tokens = conversation.total_tokens.unwrap_or(0);
        let messages = conversation.message_count.unwrap_or(0);

        Text::from(vec![
            Line::from(vec![
                Span::styled(pin_indicator, base_style),
                Span::styled(name, base_style.add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Updated: ", base_style.fg(Color::Yellow)),
                Span::styled(updated, base_style),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("Tokens: {} ", tokens),
                    base_style.fg(Color::Green),
                ),
                Span::styled(
                    format!("Messages: {}", messages),
                    base_style.fg(Color::Magenta),
                ),
            ]),
        ])
    }

    pub fn adjust_area(
        &self,
        mut area: Rect,
        max_width: u16,
        max_height: u16,
    ) -> Rect {
        area.x = area.width.saturating_sub(max_width);
        area.width = max_width;
        if area.height > max_height {
            area.height = max_height;
        }
        area
    }

    pub fn render_details(&self, frame: &mut Frame, area: Rect) {
        if let Some(selected_conversation) = self.get_current_conversation() {
            let details = vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                    Span::raw(Self::truncate_text(
                        &selected_conversation.name,
                        self.max_width() - 7,
                    )),
                ]),
                Line::from(vec![
                    Span::styled("Model: ", Style::default().fg(Color::Cyan)),
                    Span::raw(Self::truncate_text(
                        &selected_conversation.model_identifier.0,
                        self.max_width() - 8,
                    )),
                ]),
                Line::from(vec![
                    Span::styled("Updated: ", Style::default().fg(Color::Cyan)),
                    Span::raw(Self::format_timestamp(
                        selected_conversation.updated_at,
                    )),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{:?}", selected_conversation.status)),
                ]),
            ];

            let paragraph = Paragraph::new(details)
                .block(
                    Block::default()
                        .title("Details")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(Color::Cyan)),
                )
                .style(Style::default().fg(Color::White));

            frame.render_widget(paragraph, area);
        }
    }

    pub fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tabs = vec!["Active", "Archived", "Deleted"];
        let tabs = Tabs::new(tabs)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .select(match self.current_tab {
                ConversationStatus::Active => 0,
                ConversationStatus::Archived => 1,
                ConversationStatus::Deleted => 2,
            })
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::Yellow));
        frame.render_widget(tabs, area);
    }

    pub fn render_conversations_list(&mut self, frame: &mut Frame, area: Rect) {
        self.update_list_widget();
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

    pub fn max_width(&self) -> usize {
        MAX_WIDTH as usize
    }
}

#[async_trait]
impl ModalWindowTrait for ConversationListModal {
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
        tab_chat: Option<&'b mut ThreadedChatSession>,
        handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError> {
        self.handle_normal_mode_key_event(key_event, tab_chat, handler)
            .await
    }
}
