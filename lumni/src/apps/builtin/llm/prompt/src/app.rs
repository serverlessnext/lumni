use std::io;
use std::sync::Arc;

use clap::{Arg, Command};
use crossterm::cursor::Show;
use crossterm::event::{poll, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use lumni::api::env::ApplicationEnv;
use lumni::api::error::ApplicationError;
use lumni::api::spec::ApplicationSpec;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::signal;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

use super::chat::db::ConversationDatabaseStore;
use super::chat::{
    AssistantManager, ChatSession, NewConversation, PromptInstruction,
};
use super::server::{ModelServer, ModelServerName, ServerTrait};
use super::session::{prompt_app, AppSession};
pub use crate::external as lumni;

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

    let chat_session = ChatSession::new(prompt_instruction, &db_conn).await?;

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
