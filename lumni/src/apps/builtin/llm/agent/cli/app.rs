use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use crossterm::event::{
    poll, read, DisableMouseCapture, EnableMouseCapture, Event, MouseEventKind,
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

use super::prompt::ChatCompletionResponse;
use super::tui::{
    draw_ui, transition_command_line, CommandLine, PromptLogWindow,
    TextAreaHandler, TransitionAction, PromptAction,
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
    prompt_log.init().await?;

    let mut command_line = TextArea::default();
    command_line.set_cursor_line_style(Style::default());
    command_line.set_placeholder_text("Ready");

    let (tx, mut rx) = mpsc::channel(32);
    let mut tick = interval(Duration::from_millis(10));

    let is_running = Arc::new(AtomicBool::new(false));
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

                if poll(Duration::from_millis(10))? {
                    let event = read()?;

                    match event {
                        Event::Key(key_event) => {
                            let input: Input = key_event.into();
                            current_mode = process_key_event(
                                input,
                                current_mode,
                                &mut command_line_handler,
                                &mut command_line,
                                &mut editor,
                                is_running.clone(),
                                tx.clone(),
                                &mut prompt_log,
                            ).await;
                            if current_mode == TransitionAction::Quit {
                                break;
                            }
                        },
                        Event::Mouse(mouse_event) => {
                            // TODO: should track on which window the cursor actually is
                            //let mut window = prompt_log;
                            let window = &mut prompt_log;
                            match mouse_event.kind {
                                MouseEventKind::ScrollUp => {
                                    window.scroll_up();
                                },
                                MouseEventKind::ScrollDown => {
                                    window.scroll_down();
                                },
                                MouseEventKind::Down(_) => {
                                    // eprintln!("Mouse down: {:?}", mouse_event);
                                },
                                _ => {} // Other mouse events are ignored
                            }
                        },
                        _ => {} // Other events are ignored
                    }
                    redraw_ui = true;   // redraw the UI after each type of event
                }
            },
            Some(response) = rx.recv() => {
                let mut final_response = false;
                let (response_content, is_final) = process_response(&response);
                prompt_log.buffer_incoming_append(&response_content);
                final_response = is_final;

                // Drain all available messages from the channel
                if !final_response {
                    while let Ok(response) = rx.try_recv() {
                        let (response_content, is_final) = process_response(&response);
                        prompt_log.buffer_incoming_append(&response_content);

                        if is_final {
                            final_response = true;
                            break;
                        }
                    }
                } 

                // after response is complete, flush buffer to make
                // the response permanent
                if final_response {
                    prompt_log.buffer_incoming_flush();
                }
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
    tx: mpsc::Sender<Bytes>,
    prompt_log: &mut PromptLogWindow<'_>,
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
                TransitionAction::Prompt(prompt_action) => {
                    let chat_session = prompt_log.chat_session();
                    match prompt_action {
                        PromptAction::Write(prompt) => {
                            // Initiate streaming if not already active
                            if !is_running.load(Ordering::SeqCst) {
                                is_running.store(true, Ordering::SeqCst);
                                chat_session
                                    .message(tx.clone(), is_running.clone(), prompt)
                                    .await;
                            }
                        },
                        PromptAction::Clear => {
                            chat_session.reset();
                        },
                        _ => {}
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
                },
                TransitionAction::Prompt(prompt_action) => {
                    let chat_session = prompt_log.chat_session();
                    match prompt_action {
                        PromptAction::Write(prompt) => {
                            // Initiate streaming if not already active
                            if !is_running.load(Ordering::SeqCst) {
                                is_running.store(true, Ordering::SeqCst);
                                chat_session
                                    .message(tx.clone(), is_running.clone(), prompt)
                                    .await;
                            }
                        },
                        PromptAction::Clear => {
                            chat_session.reset();
                        },
                        _ => {}
                    }
                    TransitionAction::EditPrompt // Switch to editor mode
                }
                _ => TransitionAction::EditPrompt,
            }
        }
    }
}

fn process_response(response: &Bytes) -> (String, bool) {
    match ChatCompletionResponse::extract_content(response) {
        Ok(chat) => (chat.content, chat.stop),
        Err(e) => (format!("Failed to parse JSON: {}", e), true)
    }
}
