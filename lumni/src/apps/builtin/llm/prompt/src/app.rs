use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use clap::{Arg, Command};
use crossterm::cursor::Show;
use crossterm::event::{
    poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use lumni::api::error::ApplicationError;
use lumni::api::spec::ApplicationSpec;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::signal;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, timeout, Duration};

use super::chat::ChatSession;
use super::server::{ModelServer, PromptInstruction, ServerTrait};
use super::session::AppSession;
use super::tui::{
    ColorScheme, CommandLineAction, KeyEventHandler, PromptAction, TabUi,
    TextWindowTrait, WindowEvent,
};
pub use crate::external as lumni;

// max number of messages to hold before backpressure is applied
// only applies to interactive mode
const CHANNEL_QUEUE_SIZE: usize = 32;

async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_session: AppSession<'_>,
) -> Result<(), ApplicationError> {
    let defaults = app_session.get_defaults().clone();

    //let default_color_scheme = app_session.get_default_color_scheme();
    let tab = app_session.get_tab_mut(0).expect("No tab found");

    let color_scheme = tab
        .color_scheme
        .unwrap_or_else(|| defaults.get_color_scheme());

    let (tx, mut rx) = mpsc::channel(CHANNEL_QUEUE_SIZE);
    let mut tick = interval(Duration::from_millis(1));
    let keep_running = Arc::new(AtomicBool::new(false));
    let mut current_mode = Some(WindowEvent::PromptWindow);
    let mut key_event_handler = KeyEventHandler::new();
    let mut redraw_ui = true;

    // Buffer to store the trimmed trailing newlines or empty spaces
    let mut trim_buffer: Option<String> = None;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if redraw_ui {
                    tab.draw_ui(terminal)?;
                    redraw_ui = false;
                }

                // set timeout to 1ms to allow for non-blocking polling
                if poll(Duration::from_millis(1))? {
                    let event = read()?;
                    match event {
                        Event::Key(key_event) => {
                            if key_event.code == KeyCode::Tab {
                                // toggle beteen prompt and response windows
                                current_mode = match current_mode {
                                    Some(WindowEvent::PromptWindow) => {
                                        if tab.ui.prompt.is_status_insert() {
                                            // tab is locked to prompt window when in insert mode
                                            Some(WindowEvent::PromptWindow)
                                        } else {
                                            tab.ui.prompt.set_status_inactive();
                                            tab.ui.response.set_status_normal();
                                            Some(WindowEvent::ResponseWindow)
                                        }
                                    }
                                    Some(WindowEvent::ResponseWindow) => {
                                        tab.ui.response.set_status_inactive();
                                        tab.ui.prompt.set_status_normal();
                                        Some(WindowEvent::PromptWindow)
                                    }
                                    Some(WindowEvent::CommandLine(_)) => {
                                        // exit command line mode
                                        tab.ui.command_line.text_empty();
                                        tab.ui.command_line.set_status_inactive();

                                        // switch to the active window,
                                        if tab.ui.response.is_active() {
                                            tab.ui.response.set_status_normal();
                                            Some(WindowEvent::ResponseWindow)
                                        } else {
                                            tab.ui.prompt.set_status_normal();
                                            Some(WindowEvent::PromptWindow)
                                        }
                                    }
                                    _ => current_mode,
                                };
                            }

                            current_mode = if let Some(mode) = current_mode {
                                key_event_handler.process_key(
                                    key_event,
                                    &mut tab.ui,
                                    &mut tab.chat,
                                    mode,
                                    keep_running.clone(),
                                ).await
                            } else {
                                None
                            };

                            match current_mode {
                                Some(WindowEvent::Quit) => {
                                    break;
                                }
                                Some(WindowEvent::Prompt(prompt_action)) => {
                                    match prompt_action {
                                        PromptAction::Write(prompt) => {
                                            // prompt should end with single newline
                                            let formatted_prompt = format!("{}\n", prompt.trim_end());

                                            tab.ui.response.text_append_with_insert(
                                                &formatted_prompt,
                                                Some(color_scheme.get_primary_style()),
                                            );
                                            tab.ui.response.text_append_with_insert(
                                                "\n",
                                                Some(Style::reset()),
                                            );

                                            tab.chat.message(tx.clone(), formatted_prompt).await?;
                                        }
                                        PromptAction::Clear => {
                                            tab.ui.response.text_empty();
                                            tab.chat.reset();
                                            trim_buffer = None;
                                        }
                                        PromptAction::Stop => {
                                            tab.chat.stop();
                                            finalize_response(&mut tab.chat, &mut tab.ui, None, &color_scheme).await?;
                                            trim_buffer = None;
                                        }
                                    }
                                    current_mode = Some(WindowEvent::PromptWindow);
                                }
                                Some(WindowEvent::CommandLine(ref action)) => {
                                    // enter command line mode
                                    if tab.ui.prompt.is_active() {
                                        tab.ui.prompt.set_status_background();
                                    } else {
                                        tab.ui.response.set_status_background();
                                    }
                                    match action {
                                        CommandLineAction::Write(prefix) => {
                                            tab.ui.command_line.set_insert_mode();
                                            tab.ui.command_line.text_set(prefix, None);
                                        }
                                        CommandLineAction::None => {}
                                    }
                                }
                                Some(WindowEvent::Modal(modal_window_type)) => {
                                    if tab.ui.needs_modal_update(modal_window_type) {
                                        tab.ui.set_new_modal(modal_window_type);
                                    }
                                }
                                _ => {}
                            }
                        },
                        Event::Mouse(mouse_event) => {
                            // TODO: should track on which window the cursor actually is
                            let window = &mut tab.ui.response;
                            match mouse_event.kind {
                                MouseEventKind::ScrollUp => {
                                    window.scroll_up();
                                },
                                MouseEventKind::ScrollDown => {
                                    window.scroll_down();
                                },
                                MouseEventKind::Down(_) => {
                                    // mouse click is ignored currently
                                    // TODO: implement mouse click for certain actions
                                    // i.e. close modal, scroll to position, set cursor, etc.
                                    continue;   // skip redraw_ui
                                },
                                _ => {
                                    // other mouse events are ignored
                                    continue;   // skip redraw_ui
                                }
                            }
                        },
                        _ => {} // Other events are ignored
                    }
                    redraw_ui = true;   // redraw the UI after each type of event
                }
            },
            Some(response_bytes) = rx.recv() => {
                log::debug!("Received response with length {:?}", response_bytes.len());
                let mut tab_ui = &mut tab.ui;
                let mut chat = &mut tab.chat;

                let start_of_stream = if trim_buffer.is_none() {
                    // new response stream started
                    log::debug!("New response stream started");
                    tab_ui.response.enable_auto_scroll();
                    true
                } else {
                    false
                };

                let (response_content, is_final, tokens_predicted) = chat.process_response(response_bytes, start_of_stream);

                let trimmed_response = if let Some(text) = response_content.as_ref() {
                    text.trim_end().to_string()
                } else {
                    "".to_string()
                };
                log::debug!("Trimmed response: {:?}", trimmed_response);

                // display content should contain previous trimmed parts,
                // with a trimmed version of the new response content
                let display_content = format!("{}{}", trim_buffer.unwrap_or("".to_string()), trimmed_response);

                if !display_content.is_empty() {
                    chat.update_last_exchange(&display_content);
                    tab_ui.response.text_append_with_insert(&display_content, Some(color_scheme.get_secondary_style()));
                }

                if is_final {
                    log::debug!("Final response received");
                    // some servers may still send events after final chat exchange
                    // e.g. for logging or metrics. These should be retrieved to ensure
                    // the stream is fully consumed and processed.
                    while let Ok(post_bytes) = rx.try_recv() {
                        chat.process_response(post_bytes, false);
                    }
                    finalize_response(&mut chat, &mut tab_ui, tokens_predicted, &color_scheme).await?;
                    trim_buffer = None;
               } else {
                    // Capture trailing whitespaces or newlines to the trim_buffer
                    // in case the trimmed part is empty space, still capture it into trim_buffer (Some("")), to indicate a stream is running
                    if let Some(text) = response_content {
                        let trailing_whitespace_start = trimmed_response.len();
                        trim_buffer = Some(text[trailing_whitespace_start..].to_string());
                    } else {
                        trim_buffer = Some("".to_string());
                    }
                }
                redraw_ui = true;
            },
        }
    }
    Ok(())
}

async fn finalize_response(
    chat: &mut ChatSession,
    tab_ui: &mut TabUi<'_>,
    tokens_predicted: Option<usize>,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    // stop trying to get more responses
    chat.stop();
    // finalize with newline for in display
    tab_ui.response.text_append_with_insert(
        "\n",
        Some(color_scheme.get_secondary_style()),
    );
    // add an empty unstyled line
    tab_ui
        .response
        .text_append_with_insert("\n", Some(Style::reset()));
    // trim exchange + update token length
    chat.finalize_last_exchange(tokens_predicted).await?;
    Ok(())
}

fn parse_cli_arguments(spec: ApplicationSpec) -> Command {
    let name = Box::leak(spec.name().into_boxed_str()) as &'static str;
    let version = Box::leak(spec.version().into_boxed_str()) as &'static str;

    Command::new(name)
        .version(version)
        .about("CLI for prompt interaction")
        .arg_required_else_help(false)
        .arg(
            Arg::new("system")
                .long("system")
                .short('s')
                .help("System prompt"),
        )
        .arg(
            Arg::new("assistant")
                .long("assistant")
                .short('a')
                .help("Specify an assistant to use"),
        )
        .arg(
            Arg::new("server")
                .long("server")
                .short('S')
                .help("Server to use for processing the request"),
        )
        .arg(Arg::new("options").long("options").short('o').help(
            "Comma-separated list of model options e.g., \
             temperature=1,max_tokens=100",
        ))
}

pub async fn run_cli(
    spec: ApplicationSpec,
    args: Vec<String>,
) -> Result<(), ApplicationError> {
    let app = parse_cli_arguments(spec);
    let matches = app.try_get_matches_from(args).unwrap_or_else(|e| {
        e.exit();
    });

    // optional arguments
    let instruction = matches.get_one::<String>("system").cloned();
    let assistant = matches.get_one::<String>("assistant").cloned();
    let options = matches.get_one::<String>("options");

    let server_name = matches
        .get_one::<String>("server")
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "ollama".to_lowercase());

    // create new (un-initialized) server from requested server name
    let server = ModelServer::from_str(&server_name)?;
    let default_model = server.get_default_model().await;
    // setup prompt, server and chat session
    let prompt_instruction =
        PromptInstruction::new(instruction, assistant, options)?;
    let chat_session =
        ChatSession::new(Box::new(server), prompt_instruction, default_model)
            .await?;

    match poll(Duration::from_millis(0)) {
        Ok(_) => {
            // Starting interactive session
            let mut app_session = AppSession::new();
            app_session.add_tab(chat_session);
            interactive_mode(app_session).await
        }
        Err(_) => {
            // potential non-interactive input detected due to poll error.
            // attempt to use in non interactive mode
            process_non_interactive_input(chat_session).await
        }
    }
}

async fn interactive_mode(
    app_session: AppSession<'_>,
) -> Result<(), ApplicationError> {
    println!("Interactive mode detected. Starting interactive session:");
    let mut stdout = io::stdout().lock();

    // Enable raw mode and setup the screen
    if let Err(e) = enable_raw_mode() {
        execute!(stdout, Show, LeaveAlternateScreen)?;
        return Err(e.into());
    }

    if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
        let _ = disable_raw_mode();
        return Err(e.into());
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            let _ = disable_raw_mode();
            let _ = execute!(
                io::stdout(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
            return Err(e.into());
        }
    };

    // Run the application logic and capture the result
    let result = prompt_app(&mut terminal, app_session).await;

    // Regardless of the result, perform cleanup
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();
    result
}

async fn process_non_interactive_input(
    chat: ChatSession,
) -> Result<(), ApplicationError> {
    let chat = Arc::new(Mutex::new(chat));
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdin_input = String::new();

    // Shared state for handling Ctrl+C
    let running = Arc::new(Mutex::new(true));
    let shutdown_signal = Arc::new(Mutex::new(false));

    // Spawn a task to handle Ctrl+C with multiple signal support
    let running_clone = running.clone();
    let shutdown_signal_clone = shutdown_signal.clone();
    tokio::spawn(async move {
        handle_ctrl_c(running_clone, shutdown_signal_clone).await
    });

    // Attempt to read the first byte to determine if stdin has data
    let mut initial_buffer = [0; 1];
    if let Ok(1) = reader.read(&mut initial_buffer).await {
        stdin_input.push_str(&String::from_utf8_lossy(&initial_buffer));
        // Continue reading the rest of stdin
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            stdin_input.push_str(&line);
            stdin_input.push('\n'); // Maintain line breaks
        }

        let chat_clone = chat.clone();
        let input = stdin_input.trim_end().to_string();
        // Process the prompt
        let process_handle = tokio::spawn(async move {
            let mut chat = chat_clone.lock().await;
            chat.process_prompt(input, running.clone()).await
        });

        // Wait for the process to complete or for a shutdown signal
        loop {
            if *shutdown_signal.lock().await {
                // Shutdown signal received, set a timeout for graceful shutdown
                const GRACEFUL_SHUTDOWN_TIMEOUT: Duration =
                    Duration::from_secs(3);
                match timeout(GRACEFUL_SHUTDOWN_TIMEOUT, process_handle).await {
                    Ok(Ok(_)) => {
                        eprintln!(
                            "Processing completed successfully during \
                             shutdown."
                        );
                        return Ok(());
                    }
                    Ok(Err(e)) => {
                        eprintln!("Process error during shutdown: {}", e);
                        return Err(ApplicationError::Unexpected(format!(
                            "Process error: {}",
                            e
                        )));
                    }
                    Err(_) => {
                        eprintln!(
                            "Graceful shutdown timed out. Forcing exit..."
                        );
                        return Ok(());
                    }
                }
            }

            // Check if the process has completed naturally
            if process_handle.is_finished() {
                process_handle
                    .await
                    .map_err(|e| {
                        ApplicationError::Unexpected(format!(
                            "Join error: {}",
                            e
                        ))
                    })?
                    .map_err(|e| {
                        ApplicationError::Unexpected(format!(
                            "Process error: {}",
                            e
                        ))
                    })?;
                return Ok(());
            }

            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    } else {
        Err(ApplicationError::Unexpected(
            "Failed to read initial byte from stdin, possibly empty".into(),
        ))
    }
}

async fn handle_ctrl_c(r: Arc<Mutex<bool>>, s: Arc<Mutex<bool>>) {
    let mut count = 0;
    loop {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        count += 1;
        if count == 1 {
            eprintln!("Received Ctrl+C, initiating graceful shutdown...");
            let mut running = r.lock().await;
            *running = false;
            let mut shutdown = s.lock().await;
            *shutdown = true;
        } else {
            eprintln!("Received multiple Ctrl+C signals, forcing exit...");
            std::process::exit(1); // Force exit immediately
        }
    }
}
