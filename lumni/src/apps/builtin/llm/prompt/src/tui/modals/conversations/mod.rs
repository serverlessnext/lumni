use async_trait::async_trait;
use crossterm::event::KeyCode;
pub use lumni::Timestamp;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph,
};
use ratatui::Frame;

use super::{
    ApplicationError, ChatSession, Conversation, ConversationEvent,
    ConversationReader, KeyTrack, ModalWindowTrait, ModalWindowType,
    PromptInstruction, WindowEvent,
};
pub use crate::external as lumni;

const MAX_WIDTH: u16 = 32;
const MAX_HEIGHT: u16 = 20;

pub struct ConversationListModal {
    conversations: Vec<Conversation>,
    current_index: usize,
    scroll_offset: usize,
}

impl ConversationListModal {
    pub fn new(
        reader: &ConversationReader<'_>,
    ) -> Result<Self, ApplicationError> {
        let conversations = reader.fetch_conversation_list(100)?;
        Ok(Self {
            conversations,
            current_index: 0,
            scroll_offset: 0,
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
                Constraint::Length(8), // Height for Details box
                Constraint::Min(1),    // Remaining space for Conversations list
            ])
            .split(area);

        frame.render_widget(Clear, area);

        if let Some(selected_conversation) =
            self.conversations.get(self.current_index)
        {
            let details = vec![
                Line::from(Self::truncate_text(
                    &format!("ID: {}", selected_conversation.id.0),
                    max_width as usize - 2,
                )),
                Line::from(Self::truncate_text(
                    &format!("Name: {}", selected_conversation.name),
                    max_width as usize - 2,
                )),
                Line::from(Self::truncate_text(
                    &format!(
                        "Model: {}",
                        selected_conversation.model_identifier.0
                    ),
                    max_width as usize - 2,
                )),
                Line::from(Self::truncate_text(
                    &format!(
                        "Created: {}",
                        Self::format_timestamp(
                            selected_conversation.created_at
                        )
                    ),
                    max_width as usize - 2,
                )),
                Line::from(Self::truncate_text(
                    &format!(
                        "Updated: {}",
                        Self::format_timestamp(
                            selected_conversation.updated_at
                        )
                    ),
                    max_width as usize - 2,
                )),
                Line::from(Self::truncate_text(
                    &format!("Status: {:?}", selected_conversation.status),
                    max_width as usize - 2,
                )),
            ];

            let paragraph = Paragraph::new(details)
                .block(Block::default().title("Details").borders(Borders::ALL));

            frame.render_widget(Clear, chunks[0]);
            frame.render_widget(paragraph, chunks[0]);
        }

        let list_items: Vec<ListItem> = self
            .conversations
            .iter()
            .enumerate()
            .map(|(index, conversation)| {
                let style = if index == self.current_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("{}: ", conversation.id.0), style),
                        Span::styled(
                            Self::truncate_text(
                                &conversation.name,
                                max_width as usize - 5,
                            ),
                            style,
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("Model: "),
                        Span::styled(
                            Self::truncate_text(
                                &conversation.model_identifier.0,
                                max_width as usize - 7,
                            ),
                            style,
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!(
                                "Tokens: {} ",
                                conversation.total_tokens.unwrap_or(0)
                            ),
                            Style::default().fg(Color::Green),
                        ),
                        Span::styled(
                            format!(
                                "Messages: {}",
                                conversation.message_count.unwrap_or(0)
                            ),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]),
                ])
            })
            .collect();

        let list = List::new(list_items)
            .block(
                Block::default()
                    .title("Conversations")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Yellow),
            );

        frame.render_widget(Clear, chunks[1]);
        frame.render_stateful_widget(
            list,
            chunks[1],
            &mut ListState::default().with_selected(Some(self.current_index)),
        );
    }

    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        _tab_chat: &'a mut ChatSession,
        reader: &mut ConversationReader<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Up => {
                if self.current_index > 0 {
                    self.current_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.current_index < self.conversations.len() - 1 {
                    self.current_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(conversation) =
                    self.conversations.get(self.current_index)
                {
                    reader.set_conversation_id(conversation.id);
                    match PromptInstruction::from_reader(reader) {
                        Ok(prompt_instruction) => {
                            return Ok(Some(WindowEvent::PromptWindow(Some(
                                ConversationEvent::ContinueConversation(
                                    prompt_instruction,
                                ),
                            ))));
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            KeyCode::Esc => {
                return Ok(Some(WindowEvent::PromptWindow(None)));
            }
            _ => {}
        }
        Ok(Some(WindowEvent::Modal(ModalWindowType::ConversationList)))
    }
}
