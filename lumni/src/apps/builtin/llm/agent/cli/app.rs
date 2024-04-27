use std::error::Error;
use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tui_textarea::TextArea;

use super::prompt::{process_prompt, process_prompt_response, ChatSession};
use super::tui::{
    draw_ui, CommandLine, KeyEventHandler, PromptWindow, ResponseWindow,
    TextWindowTrait, WindowEvent,
};

async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
) -> Result<(), Box<dyn std::error::Error>> {
    //let mut editor_window = TextAreaHandler::new();
    let mut chat_session = ChatSession::new();
    chat_session.init().await?;

    let mut response_window = ResponseWindow::new();
    let mut prompt_window = PromptWindow::new();
    prompt_window.set_normal_mode();

    let mut command_line = TextArea::default();
    command_line.set_cursor_line_style(Style::default());
    command_line.set_placeholder_text("Ready");

    let (tx, mut rx) = mpsc::channel(32);
    let mut tick = interval(Duration::from_millis(10));
    let is_running = Arc::new(AtomicBool::new(false));
    let mut current_mode = WindowEvent::PromptWindow;
    let mut key_event_handler = KeyEventHandler::new();
    let mut command_line_handler = CommandLine::new();
    let mut redraw_ui = true;
    loop {
        tokio::select! {
            _ = tick.tick() => {
                if redraw_ui {
                    draw_ui(terminal, &mut prompt_window, &mut response_window, &command_line)?;
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
                                        if prompt_window.is_style_insert() {
                                            // tab is locked to prompt window when in insert mode
                                            WindowEvent::PromptWindow
                                        } else {
                                            prompt_window.set_style_inactive();
                                            response_window.set_style_normal();
                                            WindowEvent::ResponseWindow
                                        }
                                    }
                                    WindowEvent::ResponseWindow => {
                                        response_window.set_style_inactive();
                                        prompt_window.set_style_normal();
                                        WindowEvent::PromptWindow
                                    }
                                    _ => current_mode,
                                };
                            }


                            current_mode = key_event_handler.process_key(
                                key_event,
                                current_mode,
                                &mut command_line_handler,
                                &mut command_line,
                                &mut prompt_window,
                                is_running.clone(),
                                tx.clone(),
                                &mut response_window,
                                &mut chat_session,
                            ).await;
                            if current_mode == WindowEvent::Quit {
                                break;
                            }
                        },
                        Event::Mouse(mouse_event) => {
                            // TODO: should track on which window the cursor actually is
                            //let mut window = prompt_log;
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
                response_window.text_insert_add(&response_content);
                final_response = is_final;

                // Drain all available messages from the channel
                if !final_response {
                    while let Ok(response) = rx.try_recv() {
                        log::debug!("Received response: {:?}", response);
                        let (response_content, is_final) = process_prompt_response(&response);
                        response_window.text_insert_add(&response_content);

                        if is_final {
                            final_response = true;
                            break;
                        }
                    }
                }

                // after response is complete, flush buffer to make
                // the response permanent
                if final_response {
                    let answer = response_window.text_insert_commit();
                    chat_session.update_last_exchange(answer);
                }
                redraw_ui = true;
            },
        }
    }
    Ok(())
}

pub async fn run_cli(_args: Vec<String>) -> Result<(), Box<dyn Error>> {
    match poll(Duration::from_millis(0)) {
        Ok(_) => {
            // Starting interactive session
            interactive_mode().await
        }
        Err(_) => {
            // potential non-interactive input detected due to poll error.
            // attempt to use in non interactive mode
            process_non_interactive_input().await
        }
    }
}

async fn interactive_mode() -> Result<(), Box<dyn std::error::Error>> {
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
    let result = prompt_app(&mut terminal).await;

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

async fn process_non_interactive_input() -> Result<(), Box<dyn Error>> {
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
        process_prompt(stdin_input.trim_end().to_string(), keep_running).await;
    } else {
        return Err(
            "Failed to read initial byte from stdin, possibly empty".into()
        );
    }

    Ok(())
}
