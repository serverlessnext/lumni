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
    ApplicationError, ChatSession, Conversation, ConversationDbHandler,
    ConversationEvent, ConversationStatus, KeyTrack, ModalWindowTrait,
    ModalWindowType, PromptInstruction, WindowEvent,
};
pub use crate::external as lumni;

const MAX_WIDTH: u16 = 40;
const MAX_HEIGHT: u16 = 60;

pub struct ConversationListModal {
    conversations: Vec<Conversation>,
    current_index: usize,
    scroll_offset: usize,
    current_tab: ConversationStatus,
}

impl ConversationListModal {
    pub fn new(
        handler: &ConversationDbHandler<'_>,
    ) -> Result<Self, ApplicationError> {
        let conversations = handler.fetch_conversation_list(100)?;
        Ok(Self {
            conversations,
            current_index: 0,
            scroll_offset: 0,
            current_tab: ConversationStatus::Active,
        })
    }

    fn format_timestamp(timestamp: i64) -> String {
        Timestamp::epoch_to_rfc3339(timestamp / 1000)
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
                Ok(prompt_instruction) => Ok(Some(WindowEvent::PromptWindow(
                    Some(ConversationEvent::ContinueConversation(
                        prompt_instruction,
                    )),
                ))),
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
        let conversation_id = self.get_current_conversation()
            .map(|conv| (conv.id, conv.is_pinned));
    
        if let Some((id, is_pinned)) = conversation_id {
            let new_pin_status = !is_pinned;
            handler.update_conversation_pin_status(new_pin_status, Some(id))?;
        
            // Update the local list
            if let Some(conv) = self.conversations.iter_mut().find(|c| c.id == id) {
                conv.is_pinned = new_pin_status;
            }
        
            // Sort only conversations in the current tab
            let current_tab = self.current_tab;
            self.conversations.sort_by(|a, b| {
                if a.status == current_tab && b.status == current_tab {
                    b.is_pinned.cmp(&a.is_pinned).then(b.updated_at.cmp(&a.updated_at))
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        
            // Update the current_index to match the moved conversation
            self.current_index = self.conversations
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
        let conversation_id = self.get_current_conversation().map(|conv| conv.id);

        if let Some(id) = conversation_id {
            handler.permanent_delete_conversation(Some(id))?;
            self.conversations.retain(|c| c.id != id);

            let filtered_count = self.conversations.iter()
                .filter(|conv| conv.status == self.current_tab)
                .count();

            if filtered_count == 0 {
                // Switch to Active tab if current tab is empty
                self.current_tab = ConversationStatus::Active;
            }

            self.current_index = self.current_index.min(filtered_count.saturating_sub(1));
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
}

#[async_trait]
impl ModalWindowTrait for ConversationListModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::ConversationList
    }

    fn render_on_frame(&mut self, frame: &mut Frame, mut area: Rect) {
        let (max_width, max_height) = self.max_area_size();

        // Adjust area to be top-right
        area.x = area.width.saturating_sub(max_width);
        area.width = max_width;
        if area.height > max_height {
            area.height = max_height;
        }

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

        // Render Details box
        if let Some(selected_conversation) =
            self.conversations.get(self.current_index)
        {
            let details = vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                    Span::raw(Self::truncate_text(
                        &selected_conversation.name,
                        max_width as usize - 7,
                    )),
                ]),
                Line::from(vec![
                    Span::styled("Model: ", Style::default().fg(Color::Cyan)),
                    Span::raw(Self::truncate_text(
                        &selected_conversation.model_identifier.0,
                        max_width as usize - 8,
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

            frame.render_widget(paragraph, chunks[0]);
        }

        // Render tabs
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
        frame.render_widget(tabs, chunks[1]);

        // Render Conversations list
        let conversations_area = chunks[2];

        let filtered_conversations: Vec<&Conversation> = self
            .conversations
            .iter()
            .filter(|conv| conv.status == self.current_tab)
            .collect();

        let items: Vec<ListItem> = filtered_conversations
            .iter()
            .enumerate()
            .map(|(index, conversation)| {
                let style = if index == self.current_index {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else {
                    Style::default().bg(Color::Black).fg(Color::Cyan)
                };

                let pin_indicator = if conversation.is_pinned {
                    "📌 "
                } else {
                    "  "
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(pin_indicator, style),
                        Span::styled(
                            Self::truncate_text(
                                &conversation.name,
                                max_width as usize - 5,
                            ),
                            style,
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Updated: ", style),
                        Span::styled(
                            Self::format_timestamp(conversation.updated_at),
                            style.fg(Color::Yellow),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!(
                                "Tokens: {} ",
                                conversation.total_tokens.unwrap_or(0)
                            ),
                            style.fg(Color::Green),
                        ),
                        Span::styled(
                            format!(
                                "Messages: {}",
                                conversation.message_count.unwrap_or(0)
                            ),
                            style.fg(Color::Magenta),
                        ),
                    ]),
                ])
                .style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Conversations")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(Color::Magenta)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let mut list_state = ListState::default();
        list_state.select(Some(self.current_index));

        frame.render_stateful_widget(list, conversations_area, &mut list_state);

        // Render scrollbar
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let scrollbar_area = conversations_area.inner(&Margin {
            vertical: 1,
            horizontal: 0,
        });

        frame.render_stateful_widget(
            scrollbar,
            scrollbar_area,
            &mut ScrollbarState::new(self.conversations.len())
                .position(self.current_index),
        );

        // Render key bindings info
        let key_info = match self.current_tab {
            ConversationStatus::Active => {
                "↑↓: Navigate | Enter: Select | P: Toggle Pin | A: Archive | \
                 D: Delete | Tab: Switch Tab | Esc: Close"
            }
            ConversationStatus::Archived => {
                "↑↓: Navigate | Enter: Select | U: Unarchive | Tab: Switch Tab \
                 | Esc: Close"
            }
            ConversationStatus::Deleted => {
                "↑↓: Navigate | Enter: Select | U: Undo Delete | D: Permanent \
                 Delete | Tab: Switch Tab | Esc: Close"
            }
        };
        let key_info =
            Paragraph::new(key_info).style(Style::default().fg(Color::Cyan));
        frame.render_widget(key_info, chunks[3]);
    }

    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        _tab_chat: &'a mut ChatSession,
        handler: &mut ConversationDbHandler<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Up => {
                if self.current_index > 0 {
                    self.current_index -= 1;
                    while self.current_index > 0 && 
                          self.conversations.get(self.current_index)
                              .map_or(true, |conv| conv.status != self.current_tab) {
                        self.current_index -= 1;
                    }
                }
            }
            KeyCode::Down => {
                let filtered_count = self.conversations.iter()
                    .filter(|conv| conv.status == self.current_tab)
                    .count();
                if filtered_count > 0 && self.current_index < self.conversations.len() - 1 {
                    self.current_index += 1;
                    while self.current_index < self.conversations.len() && 
                          self.conversations[self.current_index].status != self.current_tab {
                        self.current_index += 1;
                    }
                    if self.current_index >= self.conversations.len() {
                        self.current_index = filtered_count - 1;
                    }
                }
            }
            KeyCode::Tab => {
                self.current_tab = match self.current_tab {
                    ConversationStatus::Active => ConversationStatus::Archived,
                    ConversationStatus::Archived => ConversationStatus::Deleted,
                    ConversationStatus::Deleted => ConversationStatus::Active,
                };
                self.current_index = 0;
            }
            KeyCode::Enter => {
                return self.load_conversation(handler).await;
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if self.current_tab == ConversationStatus::Active {
                    self.toggle_pin_status(handler).await?;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if self.current_tab == ConversationStatus::Active {
                    self.archive_conversation(handler).await?;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => match self.current_tab {
                ConversationStatus::Active | ConversationStatus::Archived => {
                    self.soft_delete_conversation(handler).await?
                }
                ConversationStatus::Deleted => {
                    self.permanent_delete_conversation(handler).await?
                }
            },
            KeyCode::Char('u') | KeyCode::Char('U') => match self.current_tab {
                ConversationStatus::Archived => {
                    self.unarchive_conversation(handler).await?
                }
                ConversationStatus::Deleted => {
                    self.undo_delete_conversation(handler).await?
                }
                _ => {}
            },
            KeyCode::Esc => {
                return Ok(Some(WindowEvent::PromptWindow(None)));
            }
            _ => {}
        }
        Ok(Some(WindowEvent::Modal(ModalWindowType::ConversationList)))
    }
}
