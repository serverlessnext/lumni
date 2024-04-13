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
use ratatui::backend::CrosstermBackend;
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tui_textarea::{Input, TextArea};

use super::prompt::ChatSession;
use super::tui::{
    draw_ui, transition_command_line, CommandLine, PromptLogWindow,
    TextAreaHandler, TransitionAction,
};

pub async fn run_cli(_args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut editor = TextAreaHandler::new();

    let mut prompt_log = PromptLogWindow::new();
    let chat_session = ChatSession::default();

    let mut command_line = TextArea::default();
    command_line.set_cursor_line_style(Style::default());
    command_line.set_placeholder_text("Ready");

    let (tx, mut rx) = mpsc::channel(32);
    let mut tick = interval(Duration::from_millis(10));

    let is_running = Arc::new(AtomicBool::new(true));
    let mut current_mode = TransitionAction::EditPrompt;

    let mut command_line_handler = CommandLine::new();
    let mut redraw_ui = true;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if redraw_ui {
                    draw_ui(&mut terminal, &mut editor, &mut prompt_log, &command_line)?;
                    redraw_ui = false;
                }

                if poll(Duration::from_millis(50))? {
                    let event = read()?;

                    if let Event::Key(key_event) = event {
                        let input: Input = key_event.into();
                        current_mode = process_key_event(
                            input,
                            current_mode,
                            &mut command_line_handler,
                            &mut command_line,
                            &mut editor,
                            is_running.clone(),
                            tx.clone(),
                            &chat_session,
                        ).await;
                        if current_mode == TransitionAction::Quit {
                            break;
                        }
                    }
                    redraw_ui = true;   // redraw the UI after each type of event
                }
            },
            Some(response) = rx.recv() => {
                prompt_log.insert_str(&format!("{}", response));
                redraw_ui = true;
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

async fn process_key_event(
    input: Input,
    current_mode: TransitionAction,
    command_line_handler: &mut CommandLine,
    command_line: &mut TextArea<'_>,
    editor: &mut TextAreaHandler,
    is_running: Arc<AtomicBool>,
    tx: mpsc::Sender<String>,
    chat_session: &ChatSession,
) -> TransitionAction {
    match current_mode {
        TransitionAction::CommandLine => {
            // currently in command line mode
            match transition_command_line(
                command_line_handler,
                command_line,
                editor.ta_prompt_edit(),
                input.clone(),
            )
            .await
            {
                TransitionAction::Quit => TransitionAction::Quit,
                TransitionAction::EditPrompt => TransitionAction::EditPrompt,
                TransitionAction::WritePrompt(prompt) => {
                    // Initiate streaming if not already active
                    if !is_running.load(Ordering::SeqCst) {
                        is_running.store(true, Ordering::SeqCst);
                        chat_session
                            .message(tx.clone(), is_running.clone(), prompt)
                            .await;
                    }
                    TransitionAction::EditPrompt // Switch to editor mode
                }
                _ => TransitionAction::CommandLine, // Stay in command line mode
            }
        }
        _ => {
            // editor mode
            match editor.transition(&input).await {
                TransitionAction::Quit => TransitionAction::Quit,
                TransitionAction::CommandLine => {
                    command_line.insert_str(":");
                    is_running.store(false, Ordering::SeqCst); //reset
                    TransitionAction::CommandLine
                }
                _ => TransitionAction::EditPrompt,
            }
        }
    }
}
