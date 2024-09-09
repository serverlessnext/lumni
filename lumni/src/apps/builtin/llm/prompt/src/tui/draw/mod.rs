use std::io;

use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::{Frame, Terminal};

use super::ui::{ContentDisplayMode, ConversationUi};
use super::widgets::FileBrowser;
use super::{App, TextWindowTrait, WindowMode, Workspaces};

pub async fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    window_mode: &WindowMode,
    app: &mut App<'_>,
) -> Result<(), io::Error> {
    terminal.draw(|frame| {
        let terminal_area = frame.size();
        const LIST_PANE_WIDTH: u16 = 32;
        const LIST_TAB_HEIGHT: u16 = 2;
        const WORKSPACE_NAV_HEIGHT: u16 = 2;

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
            ])
            .split(terminal_area);

        let workspace_nav_area = main_layout[0];
        let content_area = main_layout[1];

        render_workspace_nav::<B>(
            frame,
            workspace_nav_area,
            &app.ui.workspaces,
        );

        // Sub-layout for list pane and content pane
        let sub_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(LIST_PANE_WIDTH),
                Constraint::Min(0),
            ])
            .split(content_area);

        let list_pane = sub_layout[0];
        let content_pane = sub_layout[1];
        // List pane styling
        let list_block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Rgb(0, 0, 0)))
            .style(Style::default().bg(Color::Rgb(16, 24, 32)));
        frame.render_widget(list_block, list_pane);

        // Content pane styling
        let content_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(content_block, content_pane);

        // Navigation layout
        let nav_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(LIST_TAB_HEIGHT),
                Constraint::Min(0),
            ])
            .split(list_pane);

        let list_tab_area = nav_layout[0];
        let nav_content_area = nav_layout[1];

        // Render navigation tabs
        render_list_tabs::<B>(frame, list_tab_area, &app.ui.selected_mode);

        // Render navigation pane content
        match &mut app.ui.selected_mode {
            ContentDisplayMode::Conversation(_) => {
                if let Some(conversations) =
                    app.ui.workspaces.current_conversations_mut()
                {
                    conversations.render(frame, nav_content_area);
                }
            }
            ContentDisplayMode::FileBrowser(filebrowser) => {
                render_file_nav::<B>(frame, nav_content_area, filebrowser);
            }
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

        // Render modals if any
        if let Some(modal) = &mut app.ui.modal {
            let area = modal_area(terminal_area);
            modal.render_on_frame(frame, area);
        }
    })?;
    Ok(())
}

fn render_workspace_nav<B: Backend>(
    frame: &mut Frame,
    area: Rect,
    workspaces: &Workspaces,
) {
    let workspace_names: Vec<&str> = workspaces
        .workspaces
        .iter()
        .map(|w| w.name.as_str())
        .collect();

    let tabs = Tabs::new(workspace_names)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(workspaces.current_workspace_index)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn render_list_tabs<B: Backend>(
    frame: &mut Frame,
    area: Rect,
    selected_mode: &ContentDisplayMode,
) {
    let tabs = vec!["💬 Conversations", "📁 Files"];
    let tab_index = match selected_mode {
        ContentDisplayMode::Conversation(_) => 0,
        ContentDisplayMode::FileBrowser(_) => 1,
    };
    let tabs = Tabs::new(tabs)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(tab_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn render_file_nav<B: Backend>(
    frame: &mut Frame,
    area: Rect,
    file_browser: &mut FileBrowser,
) {
    frame.render_stateful_widget(
        &file_browser.widget,
        area,
        &mut file_browser.state,
    );
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

fn modal_area(area: Rect) -> Rect {
    Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(3),
        area.height.saturating_sub(4),
    )
}

fn window_hint() -> Option<String> {
    // TODO: implement window hint for main window
    None
}
