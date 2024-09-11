use std::io;

use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::{Frame, Terminal};

use super::ui::ConversationUi;
use super::{App, TextWindowTrait, WindowMode, Workspaces};

pub async fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    _window_mode: &WindowMode,
    app: &mut App<'_>,
) -> Result<(), io::Error> {
    terminal.draw(|frame| {
        let terminal_area = frame.size();
        const WORKSPACE_NAV_HEIGHT: u16 = 2;
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
                Constraint::Length(WORKSPACE_NAV_HEIGHT),
                Constraint::Min(0),
                Constraint::Length(COMMAND_LINE_HEIGHT),
            ])
            .split(terminal_area);

        let workspace_nav_area = main_layout[0];
        let content_pane = main_layout[1];
        let command_line_area = main_layout[2];

        render_workspace_nav::<B>(
            frame,
            workspace_nav_area,
            &app.ui.workspaces,
        );

        // Content pane styling
        let content_block = Block::default();
        frame.render_widget(content_block, content_pane);

        // Render active conversation stats
        if let Some(session_info) = &app.chat_manager.active_session_info {
            log::debug!("Active session info: {:?}", session_info);
        }

        // Render conversation mode
        let content_inner = content_pane.inner(Margin {
            vertical: 0,
            horizontal: 0,
        });
        render_conversation_mode::<B>(
            frame,
            content_inner,
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

fn render_workspace_nav<B: Backend>(
    frame: &mut Frame,
    area: Rect,
    workspaces: &Workspaces,
) {
    let workspace_names: Vec<String> = [].to_vec();

    let tabs = Tabs::new(workspace_names)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(workspaces.current_workspace_index)
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
    conv_ui: &mut ConversationUi,
) {
    let conversation_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(Color::Rgb(0, 0, 0)));
    frame.render_widget(conversation_block, area);

    let inner_area = area.inner(Margin {
        vertical: 0,
        horizontal: 1,
    });

    let conversation_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Min(5)])
        .split(inner_area);

    let response_area = conversation_panel[0];
    let prompt_area = conversation_panel[1];

    frame.render_widget(conv_ui.prompt.widget(&prompt_area), prompt_area);
    frame.render_widget(conv_ui.response.widget(&response_area), response_area);
}
