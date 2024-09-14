use std::io;

use lumni::Timestamp;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::{Frame, Terminal};

use super::ui::ConversationUi;
use super::{App, Conversation, TextWindowTrait, WindowMode};
pub use crate::external as lumni;

pub async fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    _window_mode: &WindowMode,
    app: &mut App<'_>,
) -> Result<(), io::Error> {
    terminal.draw(|frame| {
        let terminal_area = frame.size();
        const COMMAND_LINE_HEIGHT: u16 = 2;

        // Default background
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(16, 24, 32))),
            terminal_area,
        );

        // Main layout with workspace navigation
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(0),
                Constraint::Min(0),
                Constraint::Length(COMMAND_LINE_HEIGHT),
            ])
            .split(terminal_area);

        let content_pane = main_layout[1];
        let command_line_area = main_layout[2];

        // Content pane styling
        let content_block = Block::default();
        frame.render_widget(content_block, content_pane);

        // Render conversation mode
        let content_inner = content_pane.inner(Margin {
            vertical: 0,
            horizontal: 0,
        });
        render_conversation_mode::<B>(
            frame,
            content_inner,
            app.chat_manager.current_conversation.as_ref(),
            &mut app.ui.conversation_ui,
        );

        // Render command line
        frame.render_widget(
            app.ui.command_line.widget(&command_line_area),
            command_line_area,
        );

        // Render modals if any
        if let Some(modal) = &mut app.ui.modal {
            modal.render_on_frame(frame, terminal_area);
        }
    })?;
    Ok(())
}

fn render_workspace_nav<B: Backend>(frame: &mut Frame, area: Rect) {
    let workspace_names: Vec<String> = vec!["Chat", "Instruction"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let tabs = Tabs::new(workspace_names)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(0)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Black))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn render_conversation_mode<B: Backend>(
    frame: &mut Frame,
    area: Rect,
    current_conversation: Option<&Conversation>,
    conv_ui: &mut ConversationUi,
) {
    let conversation_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(Color::Black));
    frame.render_widget(conversation_block, area);
    let inner_area = area.inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    let conversation_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),      // Nav and info line
            Constraint::Percentage(80), // Response area
            Constraint::Min(5),         // Prompt area
        ])
        .split(inner_area);
    let nav_area = conversation_panel[0];
    let response_area = conversation_panel[1];
    let prompt_area = conversation_panel[2];

    // Render navigation area with info line
    let nav_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Chat | Instruction nav
            Constraint::Percentage(70), // Info line
        ])
        .split(nav_area);

    // Render Chat | Instruction navigation
    render_workspace_nav::<B>(frame, nav_layout[0]);

    // Render info line
    if let Some(conversation) = current_conversation {
        let info_text = Text::from(vec![Line::from(vec![
            Span::styled("Tokens: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", conversation.total_tokens.unwrap_or(0)),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" | "),
            Span::styled("Messages: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", conversation.message_count.unwrap_or(0)),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw(" | "),
            Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_timestamp(conversation.updated_at),
                Style::default().fg(Color::DarkGray),
            ),
        ])]);
        frame.render_widget(
            Paragraph::new(info_text).alignment(Alignment::Right),
            nav_layout[1],
        );
    }

    // Render response and prompt areas
    frame.render_widget(conv_ui.response.widget(&response_area), response_area);
    frame.render_widget(conv_ui.prompt.widget(&prompt_area), prompt_area);
}

fn format_timestamp(timestamp: i64) -> String {
    Timestamp::new(timestamp)
        .format("[year]-[month]-[day] [hour]:[minute]")
        .unwrap_or_else(|_| "Invalid timestamp".to_string())
}
