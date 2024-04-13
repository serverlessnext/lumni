use std::io;

use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation};
use ratatui::Terminal;
use tui_textarea::TextArea;

use super::{LayoutMode, PromptLogWindow, TextAreaHandler};

pub fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    editor: &mut TextAreaHandler,
    prompt_log: &mut PromptLogWindow,
    command_line: &TextArea,
) -> Result<(), io::Error> {
    terminal.draw(|f| {
        let terminal_size = f.size();
        const COMMAND_LINE_HEIGHT: u16 = 3;

        let prompt_log_area;
        let prompt_edit_area;
        let prompt_log_area_scrollbar;
        let command_line_area;

        match editor.layout_mode(terminal_size) {
            LayoutMode::HorizontalSplit => {
                let response_height = 8; // minimum height for response

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(40), // max-40% space for prompt (after min space is met)
                        Constraint::Min(response_height + COMMAND_LINE_HEIGHT), // command-line
                    ])
                    .split(terminal_size);

                let bottom_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(response_height), // Apply directly as no .min() available
                        Constraint::Length(COMMAND_LINE_HEIGHT),
                    ])
                    .split(chunks[1]);

                prompt_edit_area = chunks[0];
                prompt_log_area = bottom_chunks[0];
                prompt_log_area_scrollbar = chunks[1];
                command_line_area = bottom_chunks[1];
            }
            LayoutMode::VerticalSplit => {
                // Apply vertical split logic here
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0), // container for prompt_edit and prompt_log
                        Constraint::Length(COMMAND_LINE_HEIGHT), // command line
                    ])
                    .split(terminal_size);

                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(50), // left half for prompt
                        Constraint::Percentage(50), // right half for chat history
                        Constraint::Length(2),      // vertical scrollbar
                    ])
                    .split(chunks[0]);

                prompt_edit_area = main_chunks[0];
                prompt_log_area = main_chunks[1];
                prompt_log_area_scrollbar = main_chunks[2];
                command_line_area = chunks[1];
            }
        }
        f.render_widget(editor.ta_prompt_edit().widget(), prompt_edit_area);

        f.render_widget(prompt_log.widget(&prompt_log_area), prompt_log_area);
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            prompt_log_area_scrollbar,
            &mut prompt_log.vertical_scroll_state(),
        );

        f.render_widget(command_line.widget(), command_line_area);
    })?;
    Ok(())
}
