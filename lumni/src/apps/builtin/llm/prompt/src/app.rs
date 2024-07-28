use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bytes::Bytes;
use clap::{Arg, Command};
use crossterm::cursor::Show;
use crossterm::event::{
    poll, read, DisableMouseCapture, EnableMouseCapture, Event, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use lumni::api::env::ApplicationEnv;
use lumni::api::error::ApplicationError;
use lumni::api::spec::ApplicationSpec;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::signal;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, timeout, Duration};

use super::chat::db::ConversationDatabaseStore;
use super::chat::{
    AssistantManager, ChatSession, NewConversation, PromptInstruction,
};
use super::server::{ModelServer, ModelServerName, ServerTrait};
use super::session::{AppSession, TabSession};
use super::tui::{
    ColorScheme, CommandLineAction, ConversationEvent, KeyEventHandler,
    PromptAction, TabUi, TextWindowTrait, WindowEvent, WindowKind,
};
pub use crate::external as lumni;

// max number of messages to hold before backpressure is applied
// only applies to interactive mode
const CHANNEL_QUEUE_SIZE: usize = 32;

async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_session: AppSession<'_>,
    db_conn: ConversationDatabaseStore,
) -> Result<(), ApplicationError> {
    let tab = app_session.get_tab_mut(0).expect("No tab found");
    let color_scheme = tab.color_scheme;

    // add types
    let (tx, mut rx): (mpsc::Sender<Bytes>, mpsc::Receiver<Bytes>) =
        mpsc::channel(CHANNEL_QUEUE_SIZE);
    let mut tick = interval(Duration::from_millis(1));
    let keep_running = Arc::new(AtomicBool::new(false));
    let mut current_mode = Some(WindowEvent::ResponseWindow);
    let mut key_event_handler = KeyEventHandler::new();
    let mut redraw_ui = true;

    let mut reader =
        db_conn.get_conversation_reader(tab.chat.get_conversation_id());

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
                            current_mode = if let Some(mode) = current_mode {
                                key_event_handler.process_key(
                                    key_event,
                                    &mut tab.ui,
                                    &mut tab.chat,
                                    mode,
                                    keep_running.clone(),
                                    &mut reader,
                                ).await?
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
                                            send_prompt(tab, &prompt, &color_scheme, tx.clone()).await?;
                                        }
                                        PromptAction::Clear => {
                                            tab.ui.response.text_empty();
                                            tab.chat.reset(&db_conn);
                                            trim_buffer = None;
                                        }
                                        PromptAction::Stop => {
                                            tab.chat.stop();
                                            finalize_response(&mut tab.chat, &mut tab.ui, &db_conn, None, &color_scheme).await?;
                                            trim_buffer = None;
                                        }
                                    }
                                    current_mode = Some(tab.ui.set_prompt_window(false));
                                }
                                Some(WindowEvent::CommandLine(ref action)) => {
                                    // enter command line mode
                                    if tab.ui.prompt.is_active() {
                                        tab.ui.prompt.set_status_background();
                                    } else {
                                        tab.ui.response.set_status_background();
                                    }
                                    match action {
                                        Some(CommandLineAction::Write(prefix)) => {
                                            tab.ui.command_line.set_status_insert();
                                            tab.ui.command_line.text_set(prefix, None)?;
                                        }
                                        None => {}
                                    }
                                }
                                Some(WindowEvent::Modal(modal_window_type)) => {

                                    if tab.ui.needs_modal_update(modal_window_type) {
                                        tab.ui.set_new_modal(modal_window_type, &reader)?;
                                    }
                                }
                                Some(WindowEvent::PromptWindow(ref event)) => {
                                    match event {
                                        Some(ConversationEvent::NewConversation(new_conversation)) => {
                                            let prompt_instruction = PromptInstruction::new(
                                                new_conversation.clone(),
                                                &db_conn,
                                            )?;
                                            let chat_session = ChatSession::new(
                                                Some(&new_conversation.server.to_string()),
                                                prompt_instruction,
                                                &db_conn,
                                            ).await?;
                                            // stop current chat session
                                            tab.chat.stop();
                                            // update tab with new chat session
                                            tab.new_conversation(chat_session);
                                            reader = db_conn.get_conversation_reader(tab.chat.get_conversation_id());
                                        }
                                        Some(ConversationEvent::ContinueConversation(prompt_instruction)) => {
                                            let chat_session = ChatSession::new(
                                                None,
                                                (*prompt_instruction).clone(),
                                                &db_conn,
                                            ).await?;
                                            // stop current chat session
                                            tab.chat.stop();
                                            // update tab with new chat session
                                            tab.new_conversation(chat_session);
                                            reader = db_conn.get_conversation_reader(tab.chat.get_conversation_id());
                                        }
                                        _ => {
                                            log::debug!("Prompt window event not handled");
                                        }
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
                let response = match chat.process_response(response_bytes, start_of_stream) {
                    Ok(resp) => resp,
                    Err(e) => {
                        log::error!("Error processing response: {}", e);
                        None
                    }
                };
                let trimmed_response = match response {
                    Some(ref response) => {
                        let trimmed_response = response.get_content().trim_end().to_string();
                        log::debug!("Trimmed response: {:?}", trimmed_response);
                        trimmed_response
                    }
                    None => {
                        log::debug!("No response content");
                        "".to_string()
                    }
                };
                // display content should contain previous trimmed parts,
                // with a trimmed version of the new response content
                let display_content = format!("{}{}", trim_buffer.unwrap_or("".to_string()), trimmed_response);
                if !display_content.is_empty() {
                    chat.update_last_exchange(&display_content);
                    tab_ui.response.text_append(&display_content, Some(color_scheme.get_secondary_style()))?;
                }
                // response is final if is_final is true or response is None
                if response.as_ref().map(|r| r.is_final).unwrap_or(true) {
                    log::debug!("Final response received");
                    let mut final_stats = response.and_then(|r| r.stats);
                    // Process post-response messages
                    while let Ok(post_bytes) = rx.try_recv() {
                        log::debug!("Received post-response message");
                        match chat.process_response(post_bytes, false) {
                            Ok(Some(post_response)) => {
                                if let Some(post_stats) = post_response.stats {
                                    // Merge stats
                                    final_stats = Some(match final_stats {
                                        Some(mut stats) => {
                                            stats.merge(&post_stats);
                                            stats
                                        }
                                        None => post_stats,
                                    });
                                }
                            }
                            Ok(None) => {}
                            Err(e) => {
                                log::error!("Error processing post-response: {}", e);
                            }
                        }
                    }
                    finalize_response(
                        &mut chat,
                        &mut tab_ui,
                        &db_conn,
                        final_stats.as_ref().and_then(|s| s.tokens_predicted),
                        &color_scheme
                    ).await?;
                    trim_buffer = None;
                } else {
                    // Capture trailing whitespaces or newlines to the trim_buffer
                    // in case the trimmed part is empty space, still capture it into trim_buffer (Some("")), to indicate a stream is running
                    let text = match response {
                        Some(response) => response.get_content(),
                        None => "".to_string(),
                    };
                    if !text.is_empty() {
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
    db_conn: &ConversationDatabaseStore,
    tokens_predicted: Option<usize>,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    // stop trying to get more responses
    chat.stop();
    // finalize with newline for in display
    tab_ui
        .response
        .text_append("\n", Some(color_scheme.get_secondary_style()))?;
    // add an empty unstyled line
    tab_ui.response.text_append("\n", Some(Style::reset()))?;
    // trim exchange + update token length
    chat.finalize_last_exchange(db_conn, tokens_predicted)
        .await?;
    Ok(())
}

fn parse_cli_arguments(spec: ApplicationSpec) -> Command {
    let name = Box::leak(spec.name().into_boxed_str()) as &'static str;
    let version = Box::leak(spec.version().into_boxed_str()) as &'static str;

    Command::new(name)
        .version(version)
        .about("CLI for prompt interaction")
        .arg_required_else_help(false)
        .subcommand(
            Command::new("db")
                .about("Query the conversation database")
                .arg(
                    Arg::new("list")
                        .long("list")
                        .short('l')
                        .help("List recent conversations")
                        .num_args(0..=1)
                        .value_name("LIMIT"),
                )
                .arg(
                    Arg::new("id")
                        .long("id")
                        .short('i')
                        .help("Fetch a specific conversation by ID")
                        .num_args(1),
                ),
        )
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
    env: ApplicationEnv,
    args: Vec<String>,
) -> Result<(), ApplicationError> {
    let app = parse_cli_arguments(spec);
    let matches = app.try_get_matches_from(args).unwrap_or_else(|e| {
        e.exit();
    });

    let config_dir =
        env.get_config_dir().expect("Config directory not defined");
    let sqlite_file = config_dir.join("chat.db");
    let db_conn = ConversationDatabaseStore::new(&sqlite_file)?;

    if let Some(db_matches) = matches.subcommand_matches("db") {
        if db_matches.contains_id("list") {
            let limit = match db_matches.get_one::<String>("list") {
                Some(value) => value.parse().unwrap_or(20),
                None => 20, // Default value when --list is used without a value
            };
            return db_conn.print_conversation_list(limit).await;
        } else if let Some(id_value) = db_matches.get_one::<String>("id") {
            return db_conn.print_conversation_by_id(id_value).await;
        } else {
            return db_conn.print_last_conversation().await;
        }
    }
    // optional arguments
    let instruction = matches.get_one::<String>("system").cloned();
    let assistant = matches.get_one::<String>("assistant").cloned();
    let user_options = matches.get_one::<String>("options");
    let server_name = matches
        .get_one::<String>("server")
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "ollama".to_lowercase());

    // create new (un-initialized) server from requested server name
    let server = ModelServer::from_str(&server_name)?;
    let default_model = match server.get_default_model().await {
        Ok(model) => Some(model),
        Err(e) => {
            log::error!("Failed to get default model during startup: {}", e);
            None
        }
    };

    let assistant_manager =
        AssistantManager::new(assistant, instruction.clone())?;
    let initial_messages = assistant_manager.get_initial_messages().to_vec();
    // get default options via assistant
    let mut completion_options =
        assistant_manager.get_completion_options().clone();

    let model_server = ModelServerName::from_str(&server_name);
    completion_options.model_server = Some(model_server.clone());

    // overwrite default options with options set by the user
    if let Some(s) = user_options {
        let user_options_value = serde_json::from_str::<serde_json::Value>(s)?;
        completion_options.update(user_options_value)?;
    }
    let new_conversation = NewConversation {
        server: model_server,
        model: default_model,
        options: Some(serde_json::to_value(completion_options)?),
        system_prompt: instruction,
        initial_messages: Some(initial_messages),
        parent: None,
    };

    // check if the last conversation is the same as the new conversation, if so,
    // continue the conversation, otherwise start a new conversation
    let prompt_instruction = db_conn
        .fetch_last_conversation_id()?
        .and_then(|conversation_id| {
            let reader = db_conn.get_conversation_reader(Some(conversation_id));
            // Convert Result to Option using .ok()
            if new_conversation.is_equal(&reader).ok()? {
                log::debug!("Continuing last conversation");
                Some(PromptInstruction::from_reader(&reader))
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            log::debug!("Starting new conversation");
            PromptInstruction::new(new_conversation, &db_conn)
        })?;

    let chat_session =
        ChatSession::new(Some(&server_name), prompt_instruction, &db_conn)
            .await?;

    match poll(Duration::from_millis(0)) {
        Ok(_) => {
            // Starting interactive session
            let mut app_session = AppSession::new()?;
            app_session.add_tab(chat_session);
            interactive_mode(app_session, db_conn).await
        }
        Err(_) => {
            // potential non-interactive input detected due to poll error.
            // attempt to use in non interactive mode
            process_non_interactive_input(chat_session, db_conn).await
        }
    }
}

async fn interactive_mode(
    app_session: AppSession<'_>,
    db_conn: ConversationDatabaseStore,
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
    let result = prompt_app(&mut terminal, app_session, db_conn).await;

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
    _db_conn: ConversationDatabaseStore,
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

async fn send_prompt<'a>(
    tab: &mut TabSession<'a>,
    prompt: &str,
    color_scheme: &ColorScheme,
    tx: mpsc::Sender<Bytes>,
) -> Result<(), ApplicationError> {
    // prompt should end with single newline
    let formatted_prompt = format!("{}\n", prompt.trim_end());
    let result = tab.chat.message(tx.clone(), &formatted_prompt).await;

    match result {
        Ok(_) => {
            // clear prompt
            tab.ui.prompt.text_empty();
            tab.ui.prompt.set_status_normal();
            tab.ui.response.text_append(
                &formatted_prompt,
                Some(color_scheme.get_primary_style()),
            )?;
            tab.ui.set_primary_window(WindowKind::ResponseWindow);
            tab.ui.response.text_append("\n", Some(Style::reset()))?;
        }
        Err(e) => {
            log::error!("Error sending message: {:?}", e);
            tab.ui.command_line.set_alert(&e.to_string())?;
        }
    }
    Ok(())
}
