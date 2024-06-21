use std::error::Error;
use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use clap::builder::PossibleValuesParser;
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
use lumni::api::spec::ApplicationSpec;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use super::chat::{list_assistants, ChatSession, PromptInstruction};
use super::defaults::PromptStyle;
use super::model::{PromptModel, PromptModelTrait};
use super::server::ModelServer;
use super::tui::{
    draw_ui, AppUi, CommandLineAction, KeyEventHandler, PromptAction,
    TextWindowTrait, WindowEvent,
};
pub use crate::external as lumni;

// max number of messages to hold before backpressure is applied
// only applies to interactive mode
const CHANNEL_QUEUE_SIZE: usize = 32;

async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    chat_session: &mut ChatSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app_ui = AppUi::new();
    app_ui.init();

    let (tx, mut rx) = mpsc::channel(CHANNEL_QUEUE_SIZE);
    let mut tick = interval(Duration::from_millis(1));
    let keep_running = Arc::new(AtomicBool::new(false));
    let mut current_mode = WindowEvent::PromptWindow;
    let mut key_event_handler = KeyEventHandler::new();
    let mut redraw_ui = true;

    // Buffer to store the trimmed trailing newlines or empty spaces
    let mut trim_buffer: Option<String> = None;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if redraw_ui {
                    draw_ui(terminal, &mut app_ui)?;
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
                                    WindowEvent::PromptWindow => {
                                        if app_ui.prompt.is_status_insert() {
                                            // tab is locked to prompt window when in insert mode
                                            WindowEvent::PromptWindow
                                        } else {
                                            app_ui.prompt.set_status_inactive();
                                            app_ui.response.set_status_normal();
                                            WindowEvent::ResponseWindow
                                        }
                                    }
                                    WindowEvent::ResponseWindow => {
                                        app_ui.response.set_status_inactive();
                                        app_ui.prompt.set_status_normal();
                                        WindowEvent::PromptWindow
                                    }
                                    WindowEvent::CommandLine(_) => {
                                        // exit command line mode
                                        app_ui.command_line.text_empty();
                                        app_ui.command_line.set_status_inactive();

                                        // switch to the active window,
                                        if app_ui.response.is_active() {
                                            app_ui.response.set_status_normal();
                                            WindowEvent::ResponseWindow
                                        } else {
                                            app_ui.prompt.set_status_normal();
                                            WindowEvent::PromptWindow
                                        }
                                    }
                                    _ => current_mode,
                                };
                            }

                            current_mode = key_event_handler.process_key(
                                key_event,
                                &mut app_ui,
                                current_mode,
                                keep_running.clone(),
                            ).await;

                            match current_mode {
                                WindowEvent::Quit => {
                                    break;
                                }
                                WindowEvent::Prompt(prompt_action) => {
                                    match prompt_action {
                                        PromptAction::Write(prompt) => {
                                            // prompt should end with single newline
                                            let formatted_prompt = format!("{}\n", prompt.trim_end());

                                            app_ui.response.text_append_with_insert(
                                                &formatted_prompt,
                                                Some(PromptStyle::user()),
                                            );
                                            app_ui.response.text_append_with_insert(
                                                "\n",
                                                Some(Style::reset()),
                                            );

                                            chat_session.message(tx.clone(), formatted_prompt).await?;
                                        }
                                        PromptAction::Clear => {
                                            app_ui.response.text_empty();
                                            chat_session.reset();
                                            trim_buffer = None;
                                        }
                                        PromptAction::Stop => {
                                            chat_session.stop();
                                            finalize_response(chat_session, &mut app_ui, None).await?;
                                            trim_buffer = None;
                                        }
                                    }
                                    current_mode = WindowEvent::PromptWindow;
                                }
                                WindowEvent::CommandLine(ref action) => {
                                    // enter command line mode
                                    if app_ui.prompt.is_active() {
                                        app_ui.prompt.set_status_background();
                                    } else {
                                        app_ui.response.set_status_background();
                                    }
                                    match action {
                                        CommandLineAction::Write(prefix) => {
                                            app_ui.command_line.set_insert_mode();
                                            app_ui.command_line.text_set(prefix, None);
                                        }
                                        CommandLineAction::None => {}
                                    }
                                }
                                _ => {}
                            }
                        },
                        Event::Mouse(mouse_event) => {
                            // TODO: should track on which window the cursor actually is
                            let window = &mut app_ui.response;
                            match mouse_event.kind {
                                MouseEventKind::ScrollUp => {
                                    window.scroll_up();
                                },
                                MouseEventKind::ScrollDown => {
                                    window.scroll_down();
                                },
                                MouseEventKind::Down(_) => {
                                    //eprintln!("Mouse down: {:?}", mouse_event);
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
                log::debug!("Received response: {:?}", response);
                if trim_buffer.is_none() {
                    // new response stream started
                    app_ui.response.enable_auto_scroll();
                }

                let (response_content, is_final, tokens_predicted) = chat_session.process_response(&response);

                let trimmed_response_content = response_content.trim_end();

                // display content should contain previous trimmed parts,
                // with a trimmed version of the new response content
                let display_content = format!("{}{}", trim_buffer.unwrap_or("".to_string()), trimmed_response_content);
                chat_session.update_last_exchange(&display_content);
                app_ui.response.text_append_with_insert(&display_content, Some(PromptStyle::assistant()));

                if is_final {
                    finalize_response(chat_session, &mut app_ui, tokens_predicted).await?;
                    trim_buffer = None;
                } else {
                    // Capture trailing whitespaces or newlines to the trim_buffer
                    // in case the trimmed part is empty space, still capture it into trim_buffer (Some("")), to indicate a stream is running
                    let trailing_whitespace_start = trimmed_response_content.len();
                    trim_buffer = Some(response_content[trailing_whitespace_start..].to_string());
                }
                redraw_ui = true;
            },
        }
    }
    Ok(())
}

async fn finalize_response(
    chat_session: &mut ChatSession,
    //response_window: &mut ResponseWindow<'_>,
    app_ui: &mut AppUi<'_>,
    tokens_predicted: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    // finalize with newline for in display
    app_ui
        .response
        .text_append_with_insert("\n", Some(PromptStyle::assistant()));

    // add an empty unstyled line
    app_ui
        .response
        .text_append_with_insert("\n", Some(Style::reset()));
    // trim exchange + update token length
    chat_session
        .finalize_last_exchange(tokens_predicted)
        .await?;
    Ok(())
}

fn parse_cli_arguments(spec: ApplicationSpec) -> Command {
    let name = Box::leak(spec.name().into_boxed_str()) as &'static str;
    let version = Box::leak(spec.version().into_boxed_str()) as &'static str;

    let assistants: Vec<&'static str> = list_assistants()
        .expect("Failed to list assistants")
        .into_iter()
        .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
        .collect();

    let servers = vec!["ollama", "llama"];

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
                .help("Specify which assistant to use")
                .value_parser(PossibleValuesParser::new(&assistants)),
        )
        .arg(
            Arg::new("server")
                .long("server")
                .short('S')
                .help("Server to use for processing the request")
                .value_parser(PossibleValuesParser::new(&servers)),
        )
        .arg(
            Arg::new("model")
                .long("model")
                .short('m')
                .help("Model to use for processing the request")
                .default_value("auto"),
        )
        .arg(Arg::new("options").long("options").short('o').help(
            "Comma-separated list of model options e.g., \
             temperature=1,max_tokens=100",
        ))
}

pub async fn run_cli(
    spec: ApplicationSpec,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let app = parse_cli_arguments(spec);
    let matches = app.try_get_matches_from(args).unwrap_or_else(|e| {
        e.exit();
    });

    // set default values for required arguments
    let instruction = matches.get_one::<String>("system");
    let model_name = matches
        .get_one::<String>("model")
        .cloned()
        .unwrap_or_else(|| "llama3".to_string());
    // optional arguments
    let mut assistant = matches.get_one::<String>("assistant").cloned();
    let options = matches.get_one::<String>("options");

    // custom conflict check for system and assistant options
    if assistant.is_some() && instruction.is_some() {
        eprintln!(
            "Error: --system and --assistant options cannot be used together. \
             Please choose one."
        );
        std::process::exit(1);
    }

    if instruction.is_none() && assistant.is_none() {
        // for useful responses, there should either be a system prompt or an
        // assistant set. If none are given use the default assistant.
        assistant = Some("Default".to_string());
    }

    let server = matches
        .get_one::<String>("server")
        .cloned()
        .unwrap_or_else(|| "llama".to_string());

    let mut prompt_instruction = PromptInstruction::default();

    if let Some(json_str) = options {
        prompt_instruction
            .get_prompt_options_mut()
            .update_from_json(json_str);
        prompt_instruction
            .get_completion_options_mut()
            .update_from_json(json_str);
    }
    let model = Box::new(PromptModel::from_str(&model_name)?);
    prompt_instruction
        .get_completion_options_mut()
        .update_from_model(&*model as &dyn PromptModelTrait);

    let server = Box::new(ModelServer::from_str(&server, prompt_instruction)?);

    let mut chat_session = ChatSession::new(server, Some(model))?;
    if let Some(instruction) = instruction {
        chat_session.set_system_prompt(instruction).await?;
    }
    chat_session.set_assistant(assistant).init().await?;

    match poll(Duration::from_millis(0)) {
        Ok(_) => {
            // Starting interactive session
            interactive_mode(&mut chat_session).await
        }
        Err(_) => {
            // potential non-interactive input detected due to poll error.
            // attempt to use in non interactive mode
            process_non_interactive_input(&mut chat_session).await
        }
    }
}

async fn interactive_mode(
    chat_session: &mut ChatSession,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let result = prompt_app(&mut terminal, chat_session).await;

    // Regardless of the result, perform cleanup
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    result.map_err(Into::into)
}

async fn process_non_interactive_input(
    chat_session: &mut ChatSession,
) -> Result<(), Box<dyn Error>> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdin_input = String::new();

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

        let keep_running = Arc::new(AtomicBool::new(true));
        chat_session
            .process_prompt(stdin_input.trim_end().to_string(), keep_running)
            .await;
    } else {
        return Err(
            "Failed to read initial byte from stdin, possibly empty".into()
        );
    }

    Ok(())
}
