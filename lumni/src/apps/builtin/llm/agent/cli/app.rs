use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossterm::event::{
    poll, read, DisableMouseCapture, EnableMouseCapture, Event,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tokio::time::{self, interval, Duration};
use tui_textarea::{Input, TextArea};

use super::textarea::{
    transition_command_line, CommandLine, EditorMode, LayoutMode,
    TextAreaHandler, TransitionAction,
};

fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    editor: &mut TextAreaHandler,
    prompt_log: &mut TextArea,
    command_line: &mut TextArea,
) -> Result<(), io::Error> {
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
                        Constraint::Min(response_height + command_line_height), // Reserve space for prompt + command line
                    ])
                    .split(terminal_size);

                let bottom_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(response_height), // Apply directly as no .min() available
                        Constraint::Length(command_line_height),
                    ])
                    .split(chunks[1]);

                f.render_widget(editor.ta_prompt_edit().widget(), chunks[0]);
                f.render_widget(prompt_log.widget(), bottom_chunks[0]);
                f.render_widget(command_line.widget(), bottom_chunks[1]);
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
                f.render_widget(prompt_log.widget(), main_chunks[1]);
                f.render_widget(command_line.widget(), chunks[1]);
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
    let mut prompt_log = TextArea::default();
    prompt_log.set_block(EditorMode::ReadOnly.block());
    prompt_log.set_cursor_style(EditorMode::ReadOnly.cursor_style());
    prompt_log.set_placeholder_text("Ready");

    let mut command_line = TextArea::default();
    command_line.set_cursor_line_style(Style::default());
    command_line.set_placeholder_text("Ready");

    let (tx, mut rx) = mpsc::channel(32);
    let mut tick = interval(Duration::from_millis(100));
    let mut stream_active = false;

    let is_running = Arc::new(AtomicBool::new(true));
    let mut current_mode = TransitionAction::EditPrompt;

    let mut command_line_handler = CommandLine::new();

    draw_ui(
        &mut terminal,
        &mut editor,
        &mut prompt_log,
        &mut command_line,
    )?;
    loop {
        tokio::select! {
            _ = tick.tick() => {
                if poll(Duration::from_millis(100))? {
                    if let Event::Key(key_event) = read()? {
                        let input: Input = key_event.into();

                        current_mode = match current_mode {
                            TransitionAction::CommandLine => {
                                // currently in command line mode
                                match transition_command_line(
                                        &mut command_line_handler,
                                        &mut command_line,
                                        editor.ta_prompt_edit(),
                                        input.clone()
                                ).await {
                                    TransitionAction::Quit => {
                                        break; // Exit the loop immediately if Quit is returned
                                    },
                                    TransitionAction::EditPrompt => TransitionAction::EditPrompt,
                                    TransitionAction::WritePrompt(prompt) => {
                                        // Initiate streaming if not already active
                                        if !stream_active {
                                            is_running.store(true, Ordering::SeqCst);
                                            start_streaming(tx.clone(), is_running.clone(), prompt).await;
                                            stream_active = true;
                                        }
                                        TransitionAction::EditPrompt     // Switch to editor mode
                                    },
                                    _ => TransitionAction::CommandLine, // Stay in command line mode
                                }
                            },
                            _ => {
                                // editor mode
                                match editor.transition(&input).await {
                                    TransitionAction::Quit => {
                                        break; // Exit the loop immediately if Quit is returned
                                    },
                                    TransitionAction::CommandLine => {
                                        command_line.insert_str(":");
                                        stream_active = false; // Stop streaming if command line is activated
                                        is_running.store(false, Ordering::SeqCst); //reset
                                        TransitionAction::CommandLine
                                    },
                                    _ => TransitionAction::EditPrompt,
                                }
                            },
                        };

                        // Draw the UI updates unless Quit was handled by breaking the loop
                        draw_ui(&mut terminal, &mut editor, &mut prompt_log, &mut command_line)?;
                    }
                }
            },
            Some(response) = rx.recv() => {
                prompt_log.insert_str(&format!("{}", response));
                draw_ui(&mut terminal, &mut editor, &mut prompt_log, &mut command_line)?;
            },
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



async fn start_streaming(
    tx: mpsc::Sender<String>,
    is_running: Arc<AtomicBool>,
    prompt: String,
) {
    tokio::spawn(async move {
        // First, send the initial formatted question
        let initial_question = format!("Q: {}\nBot:", prompt);
        if tx.send(initial_question).await.is_err() {
            println!("Receiver dropped");
            return;
        }
        time::sleep(Duration::from_secs(1)).await;  // Simulate a slight delay after the question

        // Words to simulate a bot's streaming response
        let response_words = vec![
            "some", "random", "answer", "to", "simulate", "a", "streaming", "response", "from", "a", "bot",
        ];

        // Stream each word one by one
        for word in response_words {
            if !is_running.load(Ordering::SeqCst) {
                break;  // Stop sending if is_running is set to false
            }
            let mut response_text = String::from("");
            response_text.push(' ');  // Add a space before each word
            response_text.push_str(word);  // Append the word to the ongoing sentence

            if tx.send(response_text.clone()).await.is_err() {
                println!("Receiver dropped");
                return;
            }
            time::sleep(Duration::from_millis(200)).await;  // Simulate time between sending each word
        }

        // Reset is_running after completion
        is_running.store(false, Ordering::SeqCst);
    });
}

