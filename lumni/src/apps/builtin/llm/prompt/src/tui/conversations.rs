use std::collections::HashMap;

use lumni::Timestamp;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    Tabs,
};
use ratatui::{Frame, Terminal};

use super::widgets::{ListWidget, ListWidgetState};
use super::{Conversation, ConversationStatus};
pub use crate::external as lumni;

#[derive(Debug)]
pub struct Conversations {
    conversations: Vec<Conversation>,
    current_tab: ConversationStatus,
    tab_indices: HashMap<ConversationStatus, usize>,
    list_widget: ListWidget,
    list_widget_state: ListWidgetState,
}

impl Conversations {
    pub fn new(conversations: Vec<Conversation>) -> Self {
        let mut tab_indices = HashMap::new();
        tab_indices.insert(ConversationStatus::Active, 0);
        tab_indices.insert(ConversationStatus::Archived, 0);
        tab_indices.insert(ConversationStatus::Deleted, 0);

        let list_widget =
            ListWidget::new(Vec::new(), "Conversations".to_string())
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
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Height for tabs
                Constraint::Min(1),    // Remaining space for Conversations list
            ])
            .split(area);

        self.render_tabs(frame, chunks[0]);
        self.render_conversations_list(frame, chunks[1]);
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tabs = vec!["Active", "Archived", "Deleted"];
        let tabs = Tabs::new(tabs)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
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
        let name = self.truncate_text(&conversation.name, 30);
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

    fn conversations_in_current_tab(
        &self,
    ) -> impl Iterator<Item = &Conversation> {
        self.conversations
            .iter()
            .filter(|conv| conv.status == self.current_tab)
    }

    pub fn move_selection(&mut self, offset: i32) {
        self.list_widget
            .move_selection(&mut self.list_widget_state, offset);
        *self.tab_indices.get_mut(&self.current_tab).unwrap() =
            self.list_widget_state.selected_index;
    }

    pub fn switch_tab(&mut self) {
        self.current_tab = match self.current_tab {
            ConversationStatus::Active => ConversationStatus::Archived,
            ConversationStatus::Archived => ConversationStatus::Deleted,
            ConversationStatus::Deleted => ConversationStatus::Active,
        };
        let index = *self.tab_indices.get(&self.current_tab).unwrap_or(&0);
        self.list_widget_state.selected_index = index;
    }

    pub fn get_selected_conversation(&self) -> Option<&Conversation> {
        self.conversations_in_current_tab()
            .nth(self.list_widget_state.selected_index)
    }

    fn truncate_text(&self, text: &str, max_width: usize) -> String {
        if text.len() <= max_width {
            text.to_string()
        } else {
            format!("{}...", &text[..max_width - 3])
        }
    }
}

fn format_timestamp(timestamp: i64) -> String {
    Timestamp::new(timestamp)
        .format("[year]-[month]-[day] [hour]:[minute]:[second]")
        .unwrap_or_else(|_| "Invalid timestamp".to_string())
}
