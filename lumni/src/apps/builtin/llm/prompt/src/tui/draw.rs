use std::io;

use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation};
use ratatui::Terminal;

use super::components::TextWindowTrait;
use super::TabSession;

pub fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    tab: &mut TabSession,
) -> Result<(), io::Error> {
    let app_ui = &mut tab.ui;

    terminal.draw(|frame| {
        let terminal_size = frame.size();
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

        frame.render_widget(
            app_ui.prompt.widget(&prompt_edit_area),
            prompt_edit_area,
        );
        frame.render_widget(
            app_ui.response.widget(&prompt_log_area),
            prompt_log_area,
        );
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            prompt_log_area_scrollbar,
            &mut app_ui.response.vertical_scroll_bar_state(),
        );

        frame.render_widget(
            app_ui.command_line.widget(&command_line_area),
            command_line_area,
        );

        if let Some(modal) = &mut app_ui.modal {
            let height = main_window[0].height;
            let width = main_window[0].width;
            let area = Rect::new(2, 1, width - 6, height - 2);
            modal.render_on_frame(frame, area);
        }
    })?;
    Ok(())
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
