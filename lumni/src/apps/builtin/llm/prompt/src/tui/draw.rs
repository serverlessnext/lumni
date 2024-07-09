use std::io;

use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{
    Block, Borders, Scrollbar, ScrollbarOrientation,
};
use ratatui::Terminal;

use super::{TabSession, TextWindowTrait, WindowKind};

pub fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    tab: &mut TabSession,
) -> Result<(), io::Error> {
    terminal.draw(|frame| {
        let terminal_area = frame.size();
        const COMMAND_LINE_HEIGHT: u16 = 2;

        // default background for unused area
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            terminal_area,
        );

        let main_window = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0), // container for prompt_edit and prompt_log
                Constraint::Length(COMMAND_LINE_HEIGHT), // command line
            ])
            .split(terminal_area);

        // add borders to main_window[0]
        frame.render_widget(
            main_widget(tab.chat.server_name(), window_hint()),
            main_window[0],
        );

        let command_line_area = main_window[1];

        // first element is response text, second is prompt editor
        // editor: min 3 lines + 2 to account for border
        let tab_window_constraints =
            if tab.ui.primary_window == WindowKind::ResponseWindow {
                [Constraint::Percentage(80), Constraint::Min(5)]
            } else {
                [Constraint::Percentage(20), Constraint::Min(5)]
            };

        let tab_window = Layout::default()
            .direction(Direction::Vertical)
            .constraints(tab_window_constraints)
            .margin(1)
            .split(main_window[0]);

        let response_window = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(10),   // response history
                Constraint::Length(1), // vertical scrollbar
            ])
            .split(tab_window[0]);

        let prompt_window = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(10),   // prompt
                Constraint::Length(0), // vertical scrollbar (disabled)
            ])
            .horizontal_margin(1)
            .split(tab_window[1]);

        let response_text_area = response_window[0];
        let response_scrollbar = response_window[1];
        let prompt_text_area = prompt_window[0];

        frame.render_widget(
            tab.ui.prompt.widget(&prompt_text_area),
            prompt_text_area,
        );
        frame.render_widget(
            tab.ui.response.widget(&response_text_area),
            response_text_area,
        );
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            response_scrollbar,
            &mut tab.ui.response.vertical_scroll_bar_state(),
        );

        frame.render_widget(
            tab.ui.command_line.widget(&command_line_area),
            command_line_area,
        );

        if let Some(modal) = &mut tab.ui.modal {
            let area = modal_area(main_window[0]);
            modal.render_on_frame(frame, area);
        }
    })?;
    Ok(())
}

fn modal_area(area: Rect) -> Rect {
    Rect::new(2, 1, area.width - 3, area.height - 4)
}

pub fn main_widget(title: &str, hint: Option<String>) -> Block<'_> {
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
