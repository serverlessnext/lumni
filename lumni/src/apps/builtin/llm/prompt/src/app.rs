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

use super::chat::{
    list_assistants, process_prompt, process_prompt_response, ChatOptions,
    ChatSession,
};
use super::tui::{
    draw_ui, CommandLine, CommandLineAction, KeyEventHandler, PromptAction,
    PromptWindow, ResponseWindow, TextWindowTrait, WindowEvent,
};
pub use crate::external as lumni;

async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    chat_session: &mut ChatSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut response_window = ResponseWindow::new();
    let mut prompt_window = PromptWindow::new();
    prompt_window.set_normal_mode();

    let mut command_line = CommandLine::new();
    command_line.text_set_placeholder("Ready");

    let (tx, mut rx) = mpsc::channel(32);
    let mut tick = interval(Duration::from_millis(10));
    let keep_running = Arc::new(AtomicBool::new(false));
    let mut current_mode = WindowEvent::PromptWindow;
    let mut key_event_handler = KeyEventHandler::new();
    let mut redraw_ui = true;
    loop {
        tokio::select! {
            _ = tick.tick() => {
                if redraw_ui {
                    draw_ui(terminal, &mut prompt_window, &mut response_window, &mut command_line)?;
                    redraw_ui = false;
                }

                if poll(Duration::from_millis(10))? {
                    let event = read()?;
                    match event {
                        Event::Key(key_event) => {
                            if key_event.code == KeyCode::Tab {
                                // toggle beteen prompt and response windows
                                current_mode = match current_mode {
                                    WindowEvent::PromptWindow => {
                                        if prompt_window.is_status_insert() {
                                            // tab is locked to prompt window when in insert mode
                                            WindowEvent::PromptWindow
                                        } else {
                                            prompt_window.set_status_inactive();
                                            response_window.set_status_normal();
                                            WindowEvent::ResponseWindow
                                        }
                                    }
                                    WindowEvent::ResponseWindow => {
                                        response_window.set_status_inactive();
                                        prompt_window.set_status_normal();
                                        WindowEvent::PromptWindow
                                    }
                                    WindowEvent::CommandLine(_) => {
                                        // exit command line mode
                                        command_line.text_empty();
                                        command_line.set_status_inactive();

                                        // switch to the active window,
                                        if response_window.is_active() {
                                            response_window.set_status_normal();
                                            WindowEvent::ResponseWindow
                                        } else {
                                            prompt_window.set_status_normal();
                                            WindowEvent::PromptWindow
                                        }
                                    }
                                    _ => current_mode,
                                };
                            }


                            current_mode = key_event_handler.process_key(
                                key_event,
                                current_mode,
                                &mut command_line,
                                &mut prompt_window,
                                keep_running.clone(),
                                &mut response_window,
                            ).await;

                            match current_mode {
                                WindowEvent::Quit => {
                                    break;
                                }
                                WindowEvent::Prompt(prompt_action) => {
                                    match prompt_action {
                                        PromptAction::Write(prompt) => {
                                            chat_session.message(tx.clone(), prompt).await;
                                        }
                                        PromptAction::Clear => {

                                            response_window.text_empty();
                                            chat_session.reset();
                                        }
                                    }
                                    current_mode = WindowEvent::PromptWindow;
                                }
                                WindowEvent::CommandLine(ref action) => {
                                    // enter command line mode
                                    if prompt_window.is_active() {
                                        prompt_window.set_status_background();
                                    } else {
                                        response_window.set_status_background();
                                    }
                                    match action {
                                        CommandLineAction::Write(prefix) => {
                                            command_line.set_insert_mode();
                                            command_line.text_set(prefix, None);
                                        }
                                        CommandLineAction::None => {}
                                    }
                                }
                                _ => {}
                            }
                        },
                        Event::Mouse(mouse_event) => {
                            // TODO: should track on which window the cursor actually is
                            let window = &mut response_window;
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
                let mut final_response;
                log::debug!("Received response: {:?}", response);
                let (response_content, is_final) = process_prompt_response(&response);
                // use insert, so we can continue to append to the response and get
                // the final response back when committed
                let response_style = Some(Style::default());
                response_window.text_append_with_insert(&response_content, response_style);
                chat_session.update_last_exchange(&response_content);
                final_response = is_final;

                // Drain all available messages from the channel
                if !final_response {
                    while let Ok(response) = rx.try_recv() {
                        log::debug!("Received response: {:?}", response);
                        let (response_content, is_final) = process_prompt_response(&response);
                        response_window.text_append_with_insert(&response_content, response_style);
                        chat_session.update_last_exchange(&response_content);
                        if is_final {
                            final_response = true;
                            break;
                        }
                    }
                }
                redraw_ui = true;
            },
        }
    }
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
    let models = vec!["llama3"]; // TODO: expand with "auto", "chatgpt", etc

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
            Arg::new("model")
                .long("model")
                .short('m')
                .help("Model to use for processing the request")
                .value_parser(PossibleValuesParser::new(&models))
                .default_value(models[0]),
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

    // custom conflict check for system and assistant options
    if matches.contains_id("system") && matches.contains_id("assistant") {
        eprintln!(
            "Error: --system and --assistant options cannot be used together. \
             Please choose one."
        );
        std::process::exit(1);
    }

    // set default values for required arguments
    let instruction = matches
        .get_one::<String>("system")
        .cloned()
        .unwrap_or_else(|| "".to_string());
    let model = matches
        .get_one::<String>("model")
        .cloned()
        .unwrap_or_else(|| "llama3".to_string());
    // optional arguments
    let assistant = matches.get_one::<String>("assistant").cloned();
    let options =
        ChatOptions::new_from_args(matches.get_one::<String>("options"));

    let mut chat_session = ChatSession::new();
    chat_session
        .set_instruction(instruction)
        .set_assistant(assistant)
        .set_model(model)
        .set_options(options)
        .init()
        .await?;

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
        process_prompt(
            chat_session,
            stdin_input.trim_end().to_string(),
            keep_running,
        )
        .await;
    } else {
        return Err(
            "Failed to read initial byte from stdin, possibly empty".into()
        );
    }

    Ok(())
}
