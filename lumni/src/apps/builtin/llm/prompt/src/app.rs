use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use clap::{ArgMatches, Command};
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

use super::chat::db::{
    ConversationDatabase, EncryptionHandler, ModelServerName,
};
use super::chat::{
    prompt_app, App, AssistantManager, ChatEvent, NewConversation,
    PromptInstruction, ThreadedChatSession,
};
use super::cli::{
    handle_db_subcommand, handle_profile_subcommand, parse_cli_arguments,
};
use super::server::{ModelServer, ServerTrait};
use crate::external as lumni;

async fn create_prompt_instruction(
    matches: Option<&ArgMatches>,
    db_conn: &Arc<ConversationDatabase>,
) -> Result<PromptInstruction, ApplicationError> {
    let instruction =
        matches.and_then(|m| m.get_one::<String>("system").cloned());
    let assistant =
        matches.and_then(|m| m.get_one::<String>("assistant").cloned());
    let user_options = matches.and_then(|m| m.get_one::<String>("options"));
    let server_name = matches
        .and_then(|m| m.get_one::<String>("server"))
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
    let mut db_handler = db_conn.get_conversation_handler(None);
    let conversation_id = db_conn.fetch_last_conversation_id().await?;

    let prompt_instruction = if let Some(conversation_id) = conversation_id {
        db_handler.set_conversation_id(conversation_id);
        match new_conversation.is_equal(&db_handler).await {
            Ok(true) => {
                log::debug!("Continuing last conversation");
                Some(PromptInstruction::from_reader(&db_handler).await?)
            }
            Ok(_) => None,
            Err(e) => return Err(e.into()),
        }
    } else {
        None
    };

    let prompt_instruction = match prompt_instruction {
        Some(instruction) => instruction,
        None => {
            log::debug!("Starting new conversation");
            PromptInstruction::new(new_conversation, &mut db_handler).await?
        }
    };

    Ok(prompt_instruction)
}

fn get_possible_inputs(app: &Command) -> Vec<String> {
    let mut possible_inputs = Vec::new();

    // Add subcommands
    possible_inputs
        .extend(app.get_subcommands().map(|cmd| cmd.get_name().to_string()));

    // Add arguments (both short and long forms)
    for arg in app.get_arguments() {
        if let Some(short) = arg.get_short() {
            possible_inputs.push(format!("-{}", short));
        }
        if let Some(long) = arg.get_long() {
            possible_inputs.push(format!("--{}", long));
        }
    }
    // Manually add common options
    possible_inputs.extend(vec![
        "--help".to_string(),
        "-h".to_string(),
        "-v".to_string(),
        "--version".to_string(),
    ]);
    possible_inputs
}

pub async fn run_cli(
    spec: ApplicationSpec,
    env: ApplicationEnv,
    args: Vec<String>,
) -> Result<(), ApplicationError> {
    let app = parse_cli_arguments(spec);
    let (matches, input) = if args.len() > 1
        && args[0] == "prompt"
        && !get_possible_inputs(&app).contains(&args[1])
    {
        // If the command is "prompt" and the next arg doesn't match any command or arg, assume it's a question
        (None, Some(args[1..].join(" ")))
    } else if args.len() > 1 && args[0] == "-q" {
        // If the command is "-q", treat the rest as a question
        (None, Some(args[1..].join(" ")))
    } else if args.len() == 1 && args[0] == "-q" {
        // If only "-q" is provided without a question, print an error and exit
        eprintln!("Error: No question provided after -q");
        std::process::exit(1);
    } else {
        // Otherwise, parse as normal and let clap handle any errors
        let matches = app.try_get_matches_from(&args).unwrap_or_else(|e| {
            e.exit();
        });
        (Some(matches), None)
    };

    let config_dir =
        env.get_config_dir().expect("Config directory not defined");
    let sqlite_file = config_dir.join("chat.db");

    let encryption_handler =
        EncryptionHandler::new_from_path(None)?.map(Arc::new);

    let db_conn =
        Arc::new(ConversationDatabase::new(&sqlite_file, encryption_handler)?);

    let mut profile_handler = db_conn.get_profile_handler(None);
    if let Some(ref matches) = matches {
        if let Some(db_matches) = matches.subcommand_matches("db") {
            return handle_db_subcommand(db_matches, &db_conn).await;
        }
        if let Some(profile_matches) = matches.subcommand_matches("profile") {
            return handle_profile_subcommand(
                profile_matches,
                &mut profile_handler,
            )
            .await;
        }

        // Handle --profile option
        if let Some(profile_name) = matches.get_one::<String>("profile") {
            profile_handler.set_profile_name(profile_name.to_string());
        } else {
            // Use default profile if set
            if let Some(default_profile) =
                profile_handler.get_default_profile().await?
            {
                profile_handler.set_profile_name(default_profile);
            }
        }
    }

    // TODO: Add support for --profile option in the prompt command
    let prompt_instruction =
        create_prompt_instruction(matches.as_ref(), &db_conn).await?;

    match input {
        Some(question) => {
            // Question passed as argument
            log::debug!("Starting non-interactive session from argument");
            process_non_interactive_input(
                prompt_instruction,
                db_conn,
                Some(question),
            )
            .await
        }
        None => match poll(Duration::from_millis(0)) {
            Ok(_) => {
                // Starting interactive session
                log::debug!("Starting interactive session");
                interactive_mode(prompt_instruction, db_conn).await
            }
            Err(_) => {
                // potential stdin input detected due to poll error.
                // attempt to use in non interactive mode
                log::debug!("Starting non-interactive session from stdin");
                process_non_interactive_input(prompt_instruction, db_conn, None)
                    .await
            }
        },
    }
}

async fn interactive_mode(
    prompt_instruction: PromptInstruction,
    db_conn: Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
    let app = App::new(prompt_instruction, Arc::clone(&db_conn)).await?;
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
    let result = prompt_app(&mut terminal, app, db_conn).await;

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
    prompt_instruction: PromptInstruction,
    db_conn: Arc<ConversationDatabase>,
    question: Option<String>,
) -> Result<(), ApplicationError> {
    let chat = Arc::new(Mutex::new(ThreadedChatSession::new(
        prompt_instruction,
        db_conn.clone(),
    )));

    // Shared state for handling Ctrl+C
    let running = Arc::new(Mutex::new(true));
    let shutdown_signal = Arc::new(Mutex::new(false));

    // Spawn a task to handle Ctrl+C with multiple signal support
    let running_clone = running.clone();
    let shutdown_signal_clone = shutdown_signal.clone();
    tokio::spawn(async move {
        handle_ctrl_c(running_clone, shutdown_signal_clone).await
    });

    let input = if let Some(q) = question {
        q
    } else {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdin_input = String::new();

        // Attempt to read the first byte to determine if stdin has data
        let mut initial_buffer = [0; 1];
        if reader.read(&mut initial_buffer).await? == 1 {
            stdin_input.push_str(&String::from_utf8_lossy(&initial_buffer));
            // Continue reading the rest of stdin
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                stdin_input.push_str(&line);
                stdin_input.push('\n'); // Maintain line breaks
            }
            stdin_input.trim_end().to_string()
        } else {
            return Err(ApplicationError::Unexpected(
                "Failed to read initial byte from stdin, possibly empty".into(),
            ));
        }
    };

    let chat_clone = chat.clone();

    // Process the prompt
    let process_handle = tokio::spawn(async move {
        chat_clone.lock().await.message(&input).await?;

        let mut receiver = chat_clone.lock().await.subscribe();
        while let Ok(event) = receiver.recv().await {
            match event {
                ChatEvent::ResponseUpdate(content) => {
                    print!("{}", content);
                    std::io::stdout().flush().unwrap();
                }
                ChatEvent::FinalResponse => break,
                ChatEvent::Error(e) => {
                    return Err(ApplicationError::Unexpected(e));
                }
            }
        }
        Ok(())
    });

    // Wait for the process to complete or for a shutdown signal
    loop {
        if *shutdown_signal.lock().await {
            // Shutdown signal received, set a timeout for graceful shutdown
            const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
            match timeout(GRACEFUL_SHUTDOWN_TIMEOUT, process_handle).await {
                Ok(Ok(_)) => {
                    eprintln!(
                        "Processing completed successfully during shutdown."
                    );
                    chat.lock().await.stop();
                    return Ok(());
                }
                Ok(Err(e)) => {
                    eprintln!("Process error during shutdown: {}", e);
                    chat.lock().await.stop();
                    return Err(ApplicationError::Unexpected(format!(
                        "Process error: {}",
                        e
                    )));
                }
                Err(_) => {
                    eprintln!("Graceful shutdown timed out. Forcing exit...");
                    chat.lock().await.stop();
                    return Ok(());
                }
            }
        }

        // Check if the process has completed naturally
        if process_handle.is_finished() {
            process_handle
                .await
                .map_err(|e| {
                    ApplicationError::Unexpected(format!("Join error: {}", e))
                })?
                .map_err(|e| {
                    ApplicationError::Unexpected(format!(
                        "Process error: {}",
                        e
                    ))
                })?;
            chat.lock().await.stop();
            return Ok(());
        }

        // Wait a bit before checking again
        tokio::time::sleep(Duration::from_millis(100)).await;
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
