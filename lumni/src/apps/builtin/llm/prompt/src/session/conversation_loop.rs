use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bytes::Bytes;
use crossterm::event::{
    poll, read, Event, KeyEvent, MouseEvent, MouseEventKind,
};
use lumni::api::error::ApplicationError;
use ratatui::backend::Backend;
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use super::{
    AppSession, ChatSession, ColorScheme, CommandLineAction,
    CompletionResponse, ConversationDatabase, ConversationDbHandler,
    ConversationEvent, KeyEventHandler, ModalWindowType, PromptAction,
    PromptInstruction, TabSession, TabUi, TextWindowTrait, WindowEvent,
    WindowKind,
};
use crate::apps::builtin::llm::prompt::src::chat::db;
pub use crate::external as lumni;

// max number of messages to hold before backpressure is applied
// only applies to interactive mode
const CHANNEL_QUEUE_SIZE: usize = 32;

pub async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_session: AppSession<'_>,
    db_conn: ConversationDatabase,
) -> Result<(), ApplicationError> {
    let tab = app_session.get_tab_mut(0).expect("No tab found");
    let color_scheme = tab.color_scheme.clone();

    let (tx, mut rx) = mpsc::channel(CHANNEL_QUEUE_SIZE);
    let mut tick = interval(Duration::from_millis(1));
    let keep_running = Arc::new(AtomicBool::new(false));
    let mut current_mode = Some(WindowEvent::ResponseWindow);
    let mut key_event_handler = KeyEventHandler::new();
    let mut redraw_ui = true;

    let mut db_handler =
        db_conn.get_conversation_handler(tab.chat.get_conversation_id());
    let mut trim_buffer: Option<String> = None;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                handle_tick(terminal, tab, &mut redraw_ui, &mut current_mode, &mut key_event_handler, &keep_running, &mut db_handler, &color_scheme, tx.clone()).await?;
            },
            Some(response_bytes) = rx.recv() => {
                process_response(tab, &mut trim_buffer, response_bytes, &mut db_handler, &color_scheme, &mut rx, &mut redraw_ui).await?;
            },
        }

        if let Some(WindowEvent::Quit) = current_mode {
            break;
        }
    }
    Ok(())
}

async fn handle_tick<B: Backend>(
    terminal: &mut Terminal<B>,
    tab: &mut TabSession<'_>,
    redraw_ui: &mut bool,
    current_mode: &mut Option<WindowEvent>,
    key_event_handler: &mut KeyEventHandler,
    keep_running: &Arc<AtomicBool>,
    db_handler: &mut ConversationDbHandler<'_>,
    color_scheme: &ColorScheme,
    tx: mpsc::Sender<Bytes>,
) -> Result<(), ApplicationError> {
    if *redraw_ui {
        tab.draw_ui(terminal)?;
        *redraw_ui = false;
    }

    if poll(Duration::from_millis(1))? {
        let event = read()?;
        match event {
            Event::Key(key_event) => {
                handle_key_event(
                    tab,
                    current_mode,
                    key_event_handler,
                    key_event,
                    keep_running,
                    db_handler,
                    color_scheme,
                    tx,
                )
                .await?;
            }
            Event::Mouse(mouse_event) => handle_mouse_event(tab, mouse_event),
            _ => {}
        }
        *redraw_ui = true;
    }
    Ok(())
}

async fn handle_key_event(
    tab: &mut TabSession<'_>,
    current_mode: &mut Option<WindowEvent>,
    key_event_handler: &mut KeyEventHandler,
    key_event: KeyEvent,
    keep_running: &Arc<AtomicBool>,
    db_handler: &mut ConversationDbHandler<'_>,
    color_scheme: &ColorScheme,
    tx: mpsc::Sender<Bytes>,
) -> Result<(), ApplicationError> {
    *current_mode = if let Some(mode) = current_mode.take() {
        key_event_handler
            .process_key(
                key_event,
                &mut tab.ui,
                &mut tab.chat,
                mode,
                keep_running.clone(),
                db_handler,
            )
            .await?
    } else {
        None
    };
    match current_mode.as_mut() {
        Some(WindowEvent::Prompt(prompt_action)) => {
            handle_prompt_action(
                tab,
                prompt_action.clone(),
                color_scheme,
                tx,
                db_handler,
            )
            .await?;
            *current_mode = Some(tab.ui.set_prompt_window(false));
        }
        Some(WindowEvent::CommandLine(action)) => {
            handle_command_line_action(tab, action.clone());
        }
        Some(WindowEvent::Modal(modal_window_type)) => {
            handle_modal_window(tab, modal_window_type, db_handler)?;
        }
        Some(WindowEvent::PromptWindow(event)) => {
            handle_prompt_window_event(tab, event.clone(), db_handler).await?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_mouse_event(tab: &mut TabSession, mouse_event: MouseEvent) {
    let window = &mut tab.ui.response;
    match mouse_event.kind {
        MouseEventKind::ScrollUp => window.scroll_up(),
        MouseEventKind::ScrollDown => window.scroll_down(),
        MouseEventKind::Down(_) => {}
        _ => {}
    }
}

async fn handle_prompt_action(
    tab: &mut TabSession<'_>,
    prompt_action: PromptAction,
    color_scheme: &ColorScheme,
    tx: mpsc::Sender<Bytes>,
    db_handler: &mut ConversationDbHandler<'_>,
) -> Result<(), ApplicationError> {
    match prompt_action {
        PromptAction::Write(prompt) => {
            send_prompt(tab, &prompt, color_scheme, tx, db_handler).await?;
        }
        PromptAction::Clear => {
            tab.ui.response.text_empty();
            tab.chat.reset(db_handler);
        }
        PromptAction::Stop => {
            tab.chat.stop_chat_session();
            finalize_response(
                &mut tab.chat,
                &mut tab.ui,
                db_handler,
                None,
                color_scheme,
            )
            .await?;
        }
    }
    Ok(())
}

fn handle_command_line_action(
    tab: &mut TabSession,
    action: Option<CommandLineAction>,
) {
    if tab.ui.prompt.is_active() {
        tab.ui.prompt.set_status_background();
    } else {
        tab.ui.response.set_status_background();
    }
    if let Some(CommandLineAction::Write(prefix)) = action {
        tab.ui.command_line.set_status_insert();
        tab.ui
            .command_line
            .text_set(&prefix, None)
            .expect("Failed to set command line text");
    }
}

fn handle_modal_window(
    tab: &mut TabSession,
    modal_window_type: &ModalWindowType,
    handler: &ConversationDbHandler,
) -> Result<(), ApplicationError> {
    // switch to, or stay in modal window
    match modal_window_type {
        ModalWindowType::ConversationList(Some(_)) => {
            // reload chat on any conversation event
            tab.reload_conversation();
            return Ok(());
        }
        _ => {}
    }

    if tab.ui.needs_modal_update(modal_window_type) {
        tab.ui.set_new_modal(modal_window_type.clone(), handler)?;
    }
    Ok(())
}

async fn handle_prompt_window_event(
    tab: &mut TabSession<'_>,
    event: Option<ConversationEvent>,
    db_handler: &mut ConversationDbHandler<'_>,
) -> Result<(), ApplicationError> {
    // switch to prompt window
    match event {
        Some(ConversationEvent::NewConversation(new_conversation)) => {
            // TODO: handle in modal
            let prompt_instruction =
                PromptInstruction::new(new_conversation.clone(), db_handler)?;
            tab.chat.load_instruction(prompt_instruction).await?;
            tab.reload_conversation();
        }
        Some(_) => {
            // any other ConversationEvent is a reload
            tab.reload_conversation();
        }
        None => {
            log::debug!("No prompt window event to handle");
        }
    }
    // ensure modal is closed
    tab.ui.clear_modal();
    Ok(())
}

async fn process_response(
    tab: &mut TabSession<'_>,
    trim_buffer: &mut Option<String>,
    response_bytes: Bytes,
    db_handler: &mut ConversationDbHandler<'_>,
    color_scheme: &ColorScheme,
    rx: &mut mpsc::Receiver<Bytes>,
    redraw_ui: &mut bool,
) -> Result<(), ApplicationError> {
    log::debug!("Received response with length {:?}", response_bytes.len());
    let start_of_stream = trim_buffer.is_none();
    if start_of_stream {
        log::debug!("New response stream started");
        tab.ui.response.enable_auto_scroll();
    }

    let response = tab
        .chat
        .process_response(response_bytes, start_of_stream)
        .map_err(|e| {
        log::error!("Error processing response: {}", e);
        ApplicationError::from(e)
    })?;

    let trimmed_response = response
        .as_ref()
        .map(|r| r.get_content().trim_end().to_string())
        .unwrap_or_else(|| {
            log::debug!("No response content");
            String::new()
        });
    log::debug!("Trimmed response: {:?}", trimmed_response);

    let display_content = format!(
        "{}{}",
        trim_buffer.as_deref().unwrap_or(""),
        trimmed_response
    );
    if !display_content.is_empty() {
        tab.chat.update_last_exchange(&display_content);
        tab.ui.response.text_append(
            &display_content,
            Some(color_scheme.get_secondary_style()),
        )?;
    }

    if response.as_ref().map(|r| r.is_final).unwrap_or(true) {
        finalize_response_stream(tab, response, db_handler, color_scheme, rx)
            .await?;
        *trim_buffer = None;
    } else {
        update_trim_buffer(trim_buffer, &response, &trimmed_response);
    }

    *redraw_ui = true;
    Ok(())
}

fn update_trim_buffer(
    trim_buffer: &mut Option<String>,
    response: &Option<CompletionResponse>,
    trimmed_response: &str,
) {
    let text = response
        .as_ref()
        .map(|r| r.get_content())
        .unwrap_or_default();
    if !text.is_empty() {
        let trailing_whitespace_start = trimmed_response.len();
        *trim_buffer = Some(text[trailing_whitespace_start..].to_string());
    } else {
        *trim_buffer = Some(String::new());
    }
}

async fn finalize_response_stream(
    tab: &mut TabSession<'_>,
    response: Option<CompletionResponse>,
    db_handler: &mut ConversationDbHandler<'_>,
    color_scheme: &ColorScheme,
    rx: &mut mpsc::Receiver<Bytes>,
) -> Result<(), ApplicationError> {
    log::debug!("Final response received");
    let mut final_stats = response.and_then(|r| r.stats);

    // Process post-response messages
    while let Ok(post_bytes) = rx.try_recv() {
        log::debug!("Received post-response message");
        if let Ok(Some(post_response)) =
            tab.chat.process_response(post_bytes, false)
        {
            if let Some(post_stats) = post_response.stats {
                final_stats = Some(match final_stats {
                    Some(mut stats) => {
                        stats.merge(&post_stats);
                        stats
                    }
                    None => post_stats,
                });
            }
        }
    }

    finalize_response(
        &mut tab.chat,
        &mut tab.ui,
        db_handler,
        final_stats.as_ref().and_then(|s| s.tokens_predicted),
        color_scheme,
    )
    .await
}

async fn finalize_response(
    chat: &mut ChatSession,
    tab_ui: &mut TabUi<'_>,
    //db_conn: &ConversationDatabase,
    db_handler: &mut ConversationDbHandler<'_>,
    tokens_predicted: Option<usize>,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    // stop trying to get more responses
    chat.stop_chat_session();
    // finalize with newline for in display
    tab_ui
        .response
        .text_append("\n", Some(color_scheme.get_secondary_style()))?;
    // add an empty unstyled line
    tab_ui.response.text_append("\n", Some(Style::reset()))?;
    // trim exchange + update token length
    chat.finalize_last_exchange(db_handler, tokens_predicted)
        .await?;
    Ok(())
}

async fn send_prompt<'a>(
    tab: &mut TabSession<'a>,
    prompt: &str,
    color_scheme: &ColorScheme,
    tx: mpsc::Sender<Bytes>,
    db_handler: &ConversationDbHandler<'_>,
) -> Result<(), ApplicationError> {
    // prompt should end with single newline
    let formatted_prompt = format!("{}\n", prompt.trim_end());
    let result = tab
        .chat
        .message(tx.clone(), &formatted_prompt, db_handler)
        .await;

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
