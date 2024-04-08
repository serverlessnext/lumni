use std::error::Error;
use std::io;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;
use textarea::{LayoutMode, TextAreaHandler};
use tui_textarea::Input;

use super::textarea;

pub async fn run_prompter(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut editor = TextAreaHandler::new();

    loop {
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

                    f.render_widget(editor.ta_prompt().widget(), chunks[0]);
                    f.render_widget(
                        editor.ta_chat_history().widget(),
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
                        editor.ta_prompt().widget(),
                        main_chunks[0],
                    );
                    f.render_widget(
                        editor.ta_chat_history().widget(),
                        main_chunks[1],
                    );
                    f.render_widget(
                        editor.ta_command_line().widget(),
                        chunks[1],
                    );
                }
            }
        })?;

        let event = crossterm::event::read()?;
        let input: Input = event.into();

        if editor.transition(&input) {
            break;
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

//pub fn run_cli(args: Vec<String>) {
//    println!("{} CLI invoked with args: {:?}", PROGRAM_NAME, args);
//    if let Err(e) = run_main() {
//        eprintln!("Error: {}", e);
//    }
//}
