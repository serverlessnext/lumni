use std::error::Error;
use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::cursor::Show;
use crossterm::event::{
    poll, read, DisableMouseCapture, EnableMouseCapture, Event, MouseEventKind,
    KeyCode,
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
use tokio::time::{interval, timeout, Duration};
use tui_textarea::TextArea;

use super::prompt::{
    process_prompt, process_prompt_response,
};
use super::tui::{
    draw_ui, CommandLine, process_key_event,
    PromptLogWindow, TextAreaHandler, TransitionAction,
    EditorMode,
};

async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
) -> Result<(), Box<dyn Error>> {
    let mut editor_window = TextAreaHandler::new();

    let mut response_window = PromptLogWindow::new();
    response_window.init().await?;

    let mut command_line = TextArea::default();
    command_line.set_cursor_line_style(Style::default());
    command_line.set_placeholder_text("Ready");

    let (tx, mut rx) = mpsc::channel(32);
    let mut tick = interval(Duration::from_millis(10));

    let is_running = Arc::new(AtomicBool::new(false));
    let mut current_mode = TransitionAction::PromptWindow;

    let mut command_line_handler = CommandLine::new();
    let mut redraw_ui = true;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if redraw_ui {
                    draw_ui(terminal, &mut editor_window, &mut response_window, &command_line)?;
                    redraw_ui = false;
                }

                if poll(Duration::from_millis(10))? {
                    let event = read()?;

                    match event {
                        Event::Key(key_event) => {

                            if key_event.code == KeyCode::Tab {
                                // toggle beteen prompt and response windows
                                current_mode = match current_mode {
                                    TransitionAction::PromptWindow => {
                                        if editor_window.mode() == EditorMode::Insert {
                                            // tab is locked to prompt window when in insert mode
                                            TransitionAction::PromptWindow
                                        } else {
                                            editor_window.set_active(false);
                                            response_window.set_active(true);
                                            TransitionAction::ResponseWindow
                                        }
                                    }
                                    TransitionAction::ResponseWindow => {
                                        response_window.set_active(false);
                                        editor_window.set_active(true);
                                        TransitionAction::PromptWindow
                                    }
                                    _ => current_mode,
                                };
                            }


                            current_mode = process_key_event(
                                //input,
                                key_event,
                                current_mode,
                                &mut command_line_handler,
                                &mut command_line,
                                &mut editor_window,
                                is_running.clone(),
                                tx.clone(),
                                &mut response_window,
                            ).await;
                            if current_mode == TransitionAction::Quit {
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
                response_window.buffer_incoming_append(&response_content);
                final_response = is_final;

                // Drain all available messages from the channel
                if !final_response {
                    while let Ok(response) = rx.try_recv() {
                        log::debug!("Received response: {:?}", response);
                        let (response_content, is_final) = process_prompt_response(&response);
                        response_window.buffer_incoming_append(&response_content);

                        if is_final {
                            final_response = true;
                            break;
                        }
                    }
                }

                // after response is complete, flush buffer to make
                // the response permanent
                if final_response {
                    response_window.buffer_incoming_flush();
                }
                redraw_ui = true;
            },
        }
    }
    Ok(())
}

async fn read_initial_byte(
    reader: &mut BufReader<tokio::io::Stdin>,
) -> Result<Option<u8>, io::Error> {
    let mut buffer = [0; 1];
    let initial_read =
        timeout(Duration::from_millis(10), reader.read(&mut buffer)).await;

    match initial_read {
        Ok(Ok(count)) if count > 0 => {
            // Data was immediately available via stdin, likely non-interactive
            Ok(Some(buffer[0])) // Return the read byte
        }
        Ok(Ok(_)) | Err(_) => {
            // No data was read or timeout occurred, likely interactive
            Ok(None)
        }
        Ok(Err(e)) => {
            // Handle errors from the read operation
            Err(e)
        }
    }
}

pub async fn run_cli(_args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let stdin = tokio::io::stdin();

    let mut reader = BufReader::new(stdin);
    let initial_byte = read_initial_byte(&mut reader).await?;

    if let Some(byte) = initial_byte {
        let mut stdin_input = String::new();
        if let Ok(initial_char) = String::from_utf8(vec![byte]) {
            stdin_input.push_str(&initial_char);
        }

        // Continue reading from stdin
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            stdin_input.push_str(&line);
            stdin_input.push('\n'); // Maintain line breaks if needed
        }

        let keep_running = Arc::new(AtomicBool::new(true));
        process_prompt(stdin_input.trim_end().to_string(), keep_running).await;

        Ok(())
    } else {
        let mut stdout = io::stdout().lock();

        // Enable raw mode and setup the screen
        if let Err(e) = enable_raw_mode() {
            execute!(stdout, Show, LeaveAlternateScreen)?;
            return Err(e.into());
        }

        if let Err(e) =
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        {
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
}
