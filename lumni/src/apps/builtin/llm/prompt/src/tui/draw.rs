use std::io;

use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    Tabs,
};
use ratatui::{Frame, Terminal};

use super::ui::{ContentDisplayMode, ConversationUi};
use super::widgets::{FileBrowser, FileBrowserWidget};
use super::{App, ChatSessionManager, TextWindowTrait, WindowKind, WindowMode};

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
        const COMMAND_LINE_HEIGHT: u16 = 2;
        const NAV_TAB_HEIGHT: u16 = 3;
        const NAV_PANE_WIDTH: u16 = 30;

        // Default background for unused area
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            terminal_area,
        );

        let app_window = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(NAV_TAB_HEIGHT),
                Constraint::Min(0),
                Constraint::Length(COMMAND_LINE_HEIGHT),
            ])
            .split(terminal_area);

        let nav_tab_area = app_window[0];
        let main_area = app_window[1];
        let command_line_area = app_window[2];

        // Render navigation tabs
        render_nav_tabs::<B>(frame, nav_tab_area, &app.ui.selected_mode);

        // Split main area into navigation pane and content pane
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(NAV_PANE_WIDTH),
                Constraint::Min(0),
            ])
            .split(main_area);

        let nav_pane = main_layout[0];
        let content_pane = main_layout[1];

        // Render navigation pane based on selected mode
        match &mut app.ui.selected_mode {
            ContentDisplayMode::Conversation(_) => {
                render_conversation_nav::<B>(
                    frame,
                    nav_pane,
                    &app.chat_manager,
                );
            }
            ContentDisplayMode::FileBrowser(filebrowser) => {
                render_file_nav::<B>(frame, nav_pane, filebrowser);
            }
        }

        // Render content pane based on selected mode
        match &mut app.ui.selected_mode {
            ContentDisplayMode::Conversation(conv_ui) => {
                render_conversation_mode::<B>(
                    frame,
                    content_pane,
                    conv_ui,
                    &server_name,
                );
            }
            ContentDisplayMode::FileBrowser(filebrowser) => {
                render_file_content::<B>(frame, content_pane, &filebrowser);
            }
        }

        // Render command line
        frame.render_widget(
            app.ui.command_line.widget(&command_line_area),
            command_line_area,
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
    let tabs = vec!["Conversation", "File Browser"];
    let tab_index = match selected_mode {
        ContentDisplayMode::Conversation(_) => 0,
        ContentDisplayMode::FileBrowser(_) => 1,
    };
    let tabs = Tabs::new(tabs)
        .block(Block::default().borders(Borders::ALL))
        .select(tab_index)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Yellow));
    frame.render_widget(tabs, area);
}

fn render_conversation_nav<B: Backend>(
    frame: &mut Frame,
    area: Rect,
    chat_manager: &ChatSessionManager,
) {
    // Implement conversation navigation rendering here
    // For example, you could show a list of recent conversations
    let conversations = vec![
        ListItem::new("Conversation 1"),
        ListItem::new("Conversation 2"),
        ListItem::new("Conversation 3"),
    ];
    let conversations_list = List::new(conversations)
        .block(
            Block::default()
                .title("Recent Conversations")
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(conversations_list, area);
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
    frame.render_widget(content_window_block(server_name, window_hint()), area);

    let conversation_panel_constraints =
        if conv_ui.primary_window == WindowKind::ResponseWindow {
            [Constraint::Percentage(80), Constraint::Min(5)]
        } else {
            [Constraint::Percentage(20), Constraint::Min(5)]
        };

    let conversation_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints(conversation_panel_constraints)
        .margin(1)
        .split(area);

    let response_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(conversation_panel[0]);

    let prompt_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(0)])
        .horizontal_margin(1)
        .split(conversation_panel[1]);

    let response_text_area = response_area[0];
    let response_scrollbar = response_area[1];
    let prompt_text_area = prompt_area[0];

    frame.render_widget(
        conv_ui.prompt.widget(&prompt_text_area),
        prompt_text_area,
    );
    frame.render_widget(
        conv_ui.response.widget(&response_text_area),
        response_text_area,
    );
    frame.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        response_scrollbar,
        &mut conv_ui.response.vertical_scroll_bar_state(),
    );
}

fn render_file_content<B: Backend>(
    frame: &mut Frame,
    area: Rect,
    file_browser: &FileBrowser,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let path_area = layout[0];
    let content_area = layout[1];

    // Render current path
    let path = file_browser.current_path().display().to_string();
    let path_widget = Paragraph::new(path)
        .block(Block::default().title("Current Path").borders(Borders::ALL));
    frame.render_widget(path_widget, path_area);

    // Render file details or content based on selection
    if let Some(selected_file) = file_browser.get_selected_file() {
        if selected_file.is_dir() {
            let message =
                format!("Selected directory: {}", selected_file.display());
            let dir_info = Paragraph::new(message).block(
                Block::default()
                    .title("Directory Info")
                    .borders(Borders::ALL),
            );
            frame.render_widget(dir_info, content_area);
        } else {
            // For simplicity, we're just showing the file name here
            // In a real implementation, you might want to read and display the file contents
            let message = format!("Selected file: {}", selected_file.display());
            let file_info = Paragraph::new(message).block(
                Block::default().title("File Info").borders(Borders::ALL),
            );
            frame.render_widget(file_info, content_area);
        }
    } else {
        let no_selection = Paragraph::new("No file selected")
            .block(Block::default().title("File Info").borders(Borders::ALL));
        frame.render_widget(no_selection, content_area);
    }
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
