use std::error::Error;
use std::io;
use tokio::time::{self, Duration};

use crossterm::event::{self, poll, Event, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;
use tui_textarea::Input;

use super::textarea::{LayoutMode, TextAreaHandler};


fn draw_ui<B: Backend>(terminal: &mut Terminal<B>, editor: &mut TextAreaHandler) -> Result<(), io::Error> {
    terminal.draw(|f| {
        let terminal_size = f.size();

        match editor.layout_mode(terminal_size) {
            LayoutMode::HorizontalSplit => {
                // Adjust the approach here
                let response_height = 8; // minimum height for response
                let command_line_height = 2; // Height for command line

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(40), // max-40% space for prompt (after min space is met)
                        Constraint::Min(
                            response_height + command_line_height,
                        ), // Reserve space for prompt + command line
                    ])
                    .split(terminal_size);

                let bottom_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(response_height), // Apply directly as no .min() available
                        Constraint::Length(command_line_height),
                    ])
                    .split(chunks[1]);

                f.render_widget(
                    editor.ta_prompt_edit().widget(),
                    chunks[0],
                );
                f.render_widget(
                    editor.ta_prompt_log().widget(),
                    bottom_chunks[0],
                );
                f.render_widget(
                    editor.ta_command_line().widget(),
                    bottom_chunks[1],
                );
            }
            LayoutMode::VerticalSplit => {
                // Apply vertical split logic here
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0), // Main area takes all available space except for command line
                        Constraint::Length(3), // Fixed height for command line
                    ])
                    .split(terminal_size);

                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(50), // left half for prompt
                        Constraint::Percentage(50), // right half for chat history
                    ])
                    .split(chunks[0]);

                f.render_widget(
                    editor.ta_prompt_edit().widget(),
                    main_chunks[0],
                );
                f.render_widget(
                    editor.ta_prompt_log().widget(),
                    main_chunks[1],
                );
                f.render_widget(
                    editor.ta_command_line().widget(),
                    chunks[1],
                );
            }
        }
    })?;
    Ok(())
}


pub async fn run_cli(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut editor = TextAreaHandler::new();
    let mut should_update_ui = true;
    loop {
        if should_update_ui {
            draw_ui(&mut terminal, &mut editor)?;
            should_update_ui = false;
        }

        if poll(Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                let input: Input = key_event.into();
                if editor.transition(&input).await {
                    break;
                }
                // Update UI after every key event
                should_update_ui = true;    
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
