use std::io;

use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation};
use ratatui::Terminal;

use super::components::TextWindowTrait;
use super::AppUi;

pub fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    app_ui: &mut AppUi,
) -> Result<(), io::Error> {
    let prompt_window = &mut app_ui.prompt;
    let response_window = &mut app_ui.response;
    let command_line = &mut app_ui.command_line;

    terminal.draw(|f| {
        let terminal_size = f.size();
        const COMMAND_LINE_HEIGHT: u16 = 3;

        let prompt_log_area;
        let prompt_edit_area;
        let prompt_log_area_scrollbar;

        let main_window = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0), // container for prompt_edit and prompt_log
                Constraint::Length(COMMAND_LINE_HEIGHT), // command line
            ])
            .split(terminal_size);

        let command_line_area = main_window[1];

        let window = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // prompt_log
                Constraint::Percentage(30), // prompt_edit
            ])
            .split(main_window[0]);

        let log_window = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(10),   // chat history
                Constraint::Length(2), // vertical scrollbar
            ])
            .split(window[0]);

        let edit_window = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(10),   // prompt
                Constraint::Length(2), // vertical scrollbar
            ])
            .split(window[1]);

        prompt_log_area = log_window[0];
        prompt_log_area_scrollbar = log_window[1];
        prompt_edit_area = edit_window[0];

        f.render_widget(
            prompt_window.widget(&prompt_edit_area),
            prompt_edit_area,
        );
        f.render_widget(
            response_window.widget(&prompt_log_area),
            prompt_log_area,
        );
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            prompt_log_area_scrollbar,
            &mut response_window.vertical_scroll_bar_state(),
        );

        f.render_widget(
            command_line.widget(&command_line_area),
            command_line_area,
        );
    })?;
    Ok(())
}
