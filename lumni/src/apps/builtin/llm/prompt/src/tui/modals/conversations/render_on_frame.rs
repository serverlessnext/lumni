use super::*;

impl<'a> ConversationListModal<'a> {
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
        let filtered_conversations: Vec<&Conversation> = self
            .conversations
            .iter()
            .filter(|conv| conv.status == self.current_tab)
            .collect();

        let items: Vec<ListItem> = filtered_conversations
            .iter()
            .enumerate()
            .map(|(index, conversation)| {
                self.create_conversation_list_item(conversation, index)
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

        frame.render_stateful_widget(list, area, &mut list_state);

        self.render_scrollbar(frame, area);
        self.render_edit_line(frame, area);
    }

    pub fn create_conversation_list_item(
        &self,
        conversation: &Conversation,
        index: usize,
    ) -> ListItem {
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
                        self.max_width() - 5,
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
    }

    pub fn render_scrollbar(&self, frame: &mut Frame, area: Rect) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let scrollbar_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 0,
        });

        frame.render_stateful_widget(
            scrollbar,
            scrollbar_area,
            &mut ScrollbarState::new(self.conversations.len())
                .position(self.current_index),
        );
    }

    pub fn render_edit_line(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(edit_line) = &mut self.edit_name_line {
            let edit_area =
                Rect::new(area.x, area.y + area.height - 1, area.width, 1);
            frame.render_widget(Clear, edit_area);
            frame.render_widget(edit_line.widget(&edit_area), edit_area);
        }
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
                "↑↓: Navigate | Enter: Select | U: Undo Delete | D: Permanent \
                 Delete | E: Edit Name | Tab: Switch Tab | Esc: Close"
            }
        };
        let key_info =
            Paragraph::new(key_info).style(Style::default().fg(Color::Cyan));
        frame.render_widget(key_info, area);
    }

    fn max_width(&self) -> usize {
        MAX_WIDTH as usize
    }
}
