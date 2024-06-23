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
use super::server::{ModelServer, SUPPORTED_MODEL_ENDPOINTS};
use super::session::AppSession;
use super::tui::{
    CommandLineAction, KeyEventHandler, ModalConfigWindow, ModalWindowTrait,
    ModalWindowType, PromptAction, TabUi, TextWindowTrait, WindowEvent,
};
pub use crate::external as lumni;

// max number of messages to hold before backpressure is applied
// only applies to interactive mode
const CHANNEL_QUEUE_SIZE: usize = 32;

async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_session: AppSession<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let tab = app_session.get_tab_mut(0).expect("No tab found");

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
                let mut tab_ui = &mut tab.ui;
                let mut chat = &mut tab.chat;

                // set timeout to 1ms to allow for non-blocking polling
                if poll(Duration::from_millis(1))? {
                    let event = read()?;
                    match event {
                        Event::Key(key_event) => {
                            if key_event.code == KeyCode::Tab {
                                // toggle beteen prompt and response windows
                                current_mode = match current_mode {
                                    Some(WindowEvent::PromptWindow) => {
                                        if tab_ui.prompt.is_status_insert() {
                                            // tab is locked to prompt window when in insert mode
                                            Some(WindowEvent::PromptWindow)
                                        } else {
                                            tab_ui.prompt.set_status_inactive();
                                            tab_ui.response.set_status_normal();
                                            Some(WindowEvent::ResponseWindow)
                                        }
                                    }
                                    Some(WindowEvent::ResponseWindow) => {
                                        tab_ui.response.set_status_inactive();
                                        tab_ui.prompt.set_status_normal();
                                        Some(WindowEvent::PromptWindow)
                                    }
                                    Some(WindowEvent::CommandLine(_)) => {
                                        // exit command line mode
                                        tab_ui.command_line.text_empty();
                                        tab_ui.command_line.set_status_inactive();

                                        // switch to the active window,
                                        if tab_ui.response.is_active() {
                                            tab_ui.response.set_status_normal();
                                            Some(WindowEvent::ResponseWindow)
                                        } else {
                                            tab_ui.prompt.set_status_normal();
                                            Some(WindowEvent::PromptWindow)
                                        }
                                    }
                                    _ => current_mode,
                                };
                            }

                            current_mode = if let Some(mode) = current_mode {
                                key_event_handler.process_key(
                                    key_event,
                                    &mut tab_ui,
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

                                            tab_ui.response.text_append_with_insert(
                                                &formatted_prompt,
                                                Some(PromptStyle::user()),
                                            );
                                            tab_ui.response.text_append_with_insert(
                                                "\n",
                                                Some(Style::reset()),
                                            );

                                            chat.message(tx.clone(), formatted_prompt).await?;
                                        }
                                        PromptAction::Clear => {
                                            tab_ui.response.text_empty();
                                            chat.reset();
                                            trim_buffer = None;
                                        }
                                        PromptAction::Stop => {
                                            chat.stop();
                                            finalize_response(&mut chat, &mut tab_ui, None).await?;
                                            trim_buffer = None;
                                        }
                                    }
                                    current_mode = Some(WindowEvent::PromptWindow);
                                }
                                Some(WindowEvent::CommandLine(ref action)) => {
                                    // enter command line mode
                                    if tab_ui.prompt.is_active() {
                                        tab_ui.prompt.set_status_background();
                                    } else {
                                        tab_ui.response.set_status_background();
                                    }
                                    match action {
                                        CommandLineAction::Write(prefix) => {
                                            tab_ui.command_line.set_insert_mode();
                                            tab_ui.command_line.text_set(prefix, None);
                                        }
                                        CommandLineAction::None => {}
                                    }
                                }
                                Some(WindowEvent::Modal(modal_window_type)) => {
                                    if tab_ui.needs_modal_update(modal_window_type) {
                                        tab_ui.set_new_modal(modal_window_type);
                                    }
                                }
                                _ => {}
                            }
                        },
                        Event::Mouse(mouse_event) => {
                            // TODO: should track on which window the cursor actually is
                            let window = &mut tab_ui.response;
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
            Some(response) = rx.recv() => {
                log::debug!("Received response: {:?}", response);
                let mut tab_ui = &mut tab.ui;
                let mut chat = &mut tab.chat;

                if trim_buffer.is_none() {
                    // new response stream started
                    tab_ui.response.enable_auto_scroll();
                }

                let (response_content, is_final, tokens_predicted) = chat.process_response(&response);

                let trimmed_response_content = response_content.trim_end();

                // display content should contain previous trimmed parts,
                // with a trimmed version of the new response content
                let display_content = format!("{}{}", trim_buffer.unwrap_or("".to_string()), trimmed_response_content);
                chat.update_last_exchange(&display_content);
                tab_ui.response.text_append_with_insert(&display_content, Some(PromptStyle::assistant()));

                if is_final {
                    finalize_response(&mut chat, &mut tab_ui, tokens_predicted).await?;
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
    chat: &mut ChatSession,
    //response_window: &mut ResponseWindow<'_>,
    tab_ui: &mut TabUi<'_>,
    tokens_predicted: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    // finalize with newline for in display
    tab_ui
        .response
        .text_append_with_insert("\n", Some(PromptStyle::assistant()));

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

    let assistants: Vec<&'static str> = list_assistants()
        .expect("Failed to list assistants")
        .into_iter()
        .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
        .collect();

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
                .value_parser(PossibleValuesParser::new(
                    &SUPPORTED_MODEL_ENDPOINTS,
                )),
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
    let result = prompt_app(&mut terminal, app_session).await;

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
    mut chat: ChatSession,
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
        chat.process_prompt(stdin_input.trim_end().to_string(), keep_running)
            .await;
    } else {
        return Err(
            "Failed to read initial byte from stdin, possibly empty".into()
        );
    }

    Ok(())
}
