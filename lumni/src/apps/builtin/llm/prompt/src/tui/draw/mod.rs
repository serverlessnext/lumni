use std::io;

use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    Tabs,
};
use ratatui::{Frame, Terminal};

use super::ui::{ContentDisplayMode, ConversationUi};
use super::widgets::FileBrowser;
use super::{App, TextWindowTrait, WindowKind, WindowMode};

pub async fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    window_mode: &WindowMode,
    app: &mut App<'_>,
) -> Result<(), io::Error> {
    let server_name = app
        .chat_manager
        .active_session_info
        .as_ref()
        .and_then(|info| info.server_name.as_deref())
        .unwrap_or_default()
        .to_string();

    terminal.draw(|frame| {
        let terminal_area = frame.size();
        const NAV_PANE_WIDTH: u16 = 36;
        const NAV_TAB_HEIGHT: u16 = 3;

        // Default background
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(16, 24, 32))),
            terminal_area,
        );

        // Main layout
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(NAV_PANE_WIDTH),
                Constraint::Min(0),
            ])
            .split(terminal_area);

        let nav_pane = main_layout[0];
        let content_pane = main_layout[1];

        // Navigation pane styling
        let nav_block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(Color::Rgb(24, 32, 40)));
        frame.render_widget(nav_block, nav_pane);

        // Content pane styling
        let content_block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Rgb(16, 24, 32)));
        frame.render_widget(content_block, content_pane);

        // Navigation layout
        let nav_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(NAV_TAB_HEIGHT),
                Constraint::Min(0),
            ])
            .split(nav_pane);

        let nav_tab_area = nav_layout[0];
        let nav_content_area = nav_layout[1];

        // Render navigation tabs
        render_nav_tabs::<B>(frame, nav_tab_area, &app.ui.selected_mode);

        // Render navigation pane content
        match &mut app.ui.selected_mode {
            ContentDisplayMode::Conversation(_) => {
                app.ui.conversations.render(frame, nav_content_area);
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
            &server_name,
        );

        // Render modals if any
        if let Some(modal) = &mut app.ui.modal {
            let area = modal_area(terminal_area);
            modal.render_on_frame(frame, area);
        }
    })?;
    Ok(())
}

fn render_nav_tabs<B: Backend>(
    frame: &mut Frame,
    area: Rect,
    selected_mode: &ContentDisplayMode,
) {
    let tabs = vec!["💬 Conversation", "📁 File Browser"];
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
    server_name: &str,
) {
    let conversation_block = Block::default()
        .title(server_name)
        .title_alignment(Alignment::Center)
        .borders(Borders::NONE)
        .style(Style::default().bg(Color::Rgb(16, 24, 32)));
    frame.render_widget(conversation_block, area);

    let inner_area = area.inner(Margin {
        vertical: 1,
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

fn content_window_block(title: &str, hint: Option<String>) -> Block<'_> {
    let mut block = Block::default()
        .style(Style::default().bg(Color::Black))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightGreen).bg(Color::Black))
        .title(Title::from(title).alignment(Alignment::Left));

    if let Some(hint) = hint {
        let title_hint = Title::from(hint)
            .alignment(Alignment::Right)
            .position(Position::Top);
        block = block.title(title_hint)
    }
    block
}

fn window_hint() -> Option<String> {
    // TODO: implement window hint for main window
    None
}
