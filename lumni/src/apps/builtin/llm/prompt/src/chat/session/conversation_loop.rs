use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{
    poll, read, Event, KeyEvent, MouseEvent, MouseEventKind,
};
use lumni::api::error::ApplicationError;
use ratatui::backend::Backend;
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::time::{interval, Duration};

use super::db::{ConversationDatabase, ConversationDbHandler};
use super::{
    App, AppUi, ChatSession, ColorScheme, CommandLineAction,
    CompletionResponse, ConversationEvent, KeyEventHandler, ModalWindowType,
    PromptAction, PromptInstruction, TextWindowTrait, WindowEvent, WindowKind,
};
pub use crate::external as lumni;

pub async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App<'_>,
    db_conn: ConversationDatabase,
) -> Result<(), ApplicationError> {
    let color_scheme = app.color_scheme.clone();

    let mut tick = interval(Duration::from_millis(1));
    let keep_running = Arc::new(AtomicBool::new(false));
    let mut current_mode = Some(WindowEvent::ResponseWindow);
    let mut key_event_handler = KeyEventHandler::new();
    let mut redraw_ui = true;

    let mut db_handler = db_conn
        .get_conversation_handler(app.get_conversation_id_for_active_session());

    loop {
        tokio::select! {
            _ = tick.tick() => {
                handle_tick(terminal, &mut app, &mut redraw_ui, &mut current_mode, &mut key_event_handler, &keep_running, &mut db_handler, &color_scheme).await?;
            },
            result = app.chat_manager.get_active_session().receive_response() => {
                match result {
                    Ok(Some(response)) => {
                        process_chat_response(&mut app, response, &mut db_handler, &color_scheme, &mut redraw_ui).await?;
                    },
                    Ok(None) => {
                        log::debug!("Chat session channel closed");
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    },
                    Err(e) => {
                        log::error!("Error receiving response: {:?}", e);
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
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
    app: &mut App<'_>,
    redraw_ui: &mut bool,
    current_mode: &mut Option<WindowEvent>,
    key_event_handler: &mut KeyEventHandler,
    keep_running: &Arc<AtomicBool>,
    db_handler: &mut ConversationDbHandler<'_>,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    if *redraw_ui {
        app.draw_ui(terminal)?;
        *redraw_ui = false;
    }

    if poll(Duration::from_millis(1))? {
        let event = read()?;
        match event {
            Event::Key(key_event) => {
                handle_key_event(
                    app,
                    current_mode,
                    key_event_handler,
                    key_event,
                    keep_running,
                    db_handler,
                    color_scheme,
                )
                .await?;
            }
            Event::Mouse(mouse_event) => handle_mouse_event(app, mouse_event),
            _ => {}
        }
        *redraw_ui = true;
    }
    Ok(())
}

async fn handle_key_event(
    app: &mut App<'_>,
    current_mode: &mut Option<WindowEvent>,
    key_event_handler: &mut KeyEventHandler,
    key_event: KeyEvent,
    keep_running: &Arc<AtomicBool>,
    db_handler: &mut ConversationDbHandler<'_>,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    *current_mode = if let Some(mode) = current_mode.take() {
        key_event_handler
            .process_key(
                key_event,
                &mut app.ui,
                app.chat_manager.get_active_session(),
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
                app,
                prompt_action.clone(),
                color_scheme,
                db_handler,
            )
            .await?;
            *current_mode = Some(app.ui.set_prompt_window(false));
        }
        Some(WindowEvent::CommandLine(action)) => {
            handle_command_line_action(app, action.clone());
        }
        Some(WindowEvent::Modal(modal_window_type)) => {
            handle_modal_window(app, modal_window_type, db_handler)?;
        }
        Some(WindowEvent::PromptWindow(event)) => {
            handle_prompt_window_event(app, event.clone(), db_handler).await?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_mouse_event(app: &mut App, mouse_event: MouseEvent) {
    let window = &mut app.ui.response;
    match mouse_event.kind {
        MouseEventKind::ScrollUp => window.scroll_up(),
        MouseEventKind::ScrollDown => window.scroll_down(),
        MouseEventKind::Down(_) => {}
        _ => {}
    }
}

async fn handle_prompt_action(
    app: &mut App<'_>,
    prompt_action: PromptAction,
    color_scheme: &ColorScheme,
    db_handler: &mut ConversationDbHandler<'_>,
) -> Result<(), ApplicationError> {
    match prompt_action {
        PromptAction::Write(prompt) => {
            send_prompt(app, &prompt, color_scheme, db_handler).await?;
        }
        PromptAction::Clear => {
            app.ui.response.text_empty();
            app.reset_active_session(db_handler)
        }
        PromptAction::Stop => {
            app.stop_active_chat_session();
            finalize_response(
                &mut app.chat_manager.get_active_session(),
                &mut app.ui,
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
    app: &mut App,
    action: Option<CommandLineAction>,
) {
    if app.ui.prompt.is_active() {
        app.ui.prompt.set_status_background();
    } else {
        app.ui.response.set_status_background();
    }
    if let Some(CommandLineAction::Write(prefix)) = action {
        app.ui.command_line.set_status_insert();
        app.ui
            .command_line
            .text_set(&prefix, None)
            .expect("Failed to set command line text");
    }
}

fn handle_modal_window(
    app: &mut App,
    modal_window_type: &ModalWindowType,
    handler: &ConversationDbHandler,
) -> Result<(), ApplicationError> {
    // switch to, or stay in modal window
    match modal_window_type {
        ModalWindowType::ConversationList(Some(_)) => {
            // reload chat on any conversation event
            app.reload_conversation();
            return Ok(());
        }
        _ => {}
    }

    if app.ui.needs_modal_update(modal_window_type) {
        app.ui.set_new_modal(modal_window_type.clone(), handler)?;
    }
    Ok(())
}

async fn handle_prompt_window_event(
    app: &mut App<'_>,
    event: Option<ConversationEvent>,
    db_handler: &mut ConversationDbHandler<'_>,
) -> Result<(), ApplicationError> {
    // switch to prompt window
    match event {
        Some(ConversationEvent::NewConversation(new_conversation)) => {
            // TODO: handle in modal
            let prompt_instruction =
                PromptInstruction::new(new_conversation.clone(), db_handler)?;
            app.load_instruction_for_active_session(prompt_instruction)
                .await?;
            app.reload_conversation();
        }
        Some(_) => {
            // any other ConversationEvent is a reload
            app.reload_conversation();
        }
        None => {
            log::debug!("No prompt window event to handle");
        }
    }
    // ensure modal is closed
    app.ui.clear_modal();
    Ok(())
}

async fn finalize_response(
    chat: &mut ChatSession,
    app_ui: &mut AppUi<'_>,
    //db_conn: &ConversationDatabase,
    db_handler: &mut ConversationDbHandler<'_>,
    tokens_predicted: Option<usize>,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    // stop trying to get more responses
    chat.stop_chat_session();
    // finalize with newline for in display
    app_ui
        .response
        .text_append("\n", Some(color_scheme.get_secondary_style()))?;
    // add an empty unstyled line
    app_ui.response.text_append("\n", Some(Style::reset()))?;
    // trim exchange + update token length
    chat.finalize_last_exchange(db_handler, tokens_predicted)
        .await?;
    Ok(())
}

async fn send_prompt<'a>(
    app: &mut App<'a>,
    prompt: &str,
    color_scheme: &ColorScheme,
    db_handler: &ConversationDbHandler<'_>,
) -> Result<(), ApplicationError> {
    // prompt should end with single newline
    let formatted_prompt = format!("{}\n", prompt.trim_end());
    let result = app
        .chat_manager
        .get_active_session()
        .message(&formatted_prompt, db_handler)
        .await;

    match result {
        Ok(_) => {
            // clear prompt
            app.ui.prompt.text_empty();
            app.ui.prompt.set_status_normal();
            app.ui.response.text_append(
                &formatted_prompt,
                Some(color_scheme.get_primary_style()),
            )?;
            app.ui.set_primary_window(WindowKind::ResponseWindow);
            app.ui.response.text_append("\n", Some(Style::reset()))?;
        }
        Err(e) => {
            log::error!("Error sending message: {:?}", e);
            app.ui.command_line.set_alert(&e.to_string())?;
        }
    }
    Ok(())
}

async fn process_chat_response(
    app: &mut App<'_>,
    response: CompletionResponse,
    db_handler: &mut ConversationDbHandler<'_>,
    color_scheme: &ColorScheme,
    redraw_ui: &mut bool,
) -> Result<(), ApplicationError> {
    log::debug!(
        "Received response with length {:?}",
        response.get_content().len()
    );

    let trimmed_response = response.get_content().trim_end().to_string();
    log::debug!("Trimmed response: {:?}", trimmed_response);

    if !trimmed_response.is_empty() {
        app.chat_manager
            .get_active_session()
            .update_last_exchange(&trimmed_response);
        app.ui
            .response
            .text_append(
                &trimmed_response,
                Some(color_scheme.get_secondary_style()),
            )
            .map_err(|e| ApplicationError::Runtime(e.to_string()))?;
    }

    if response.is_final {
        finalize_chat_response(app, response, db_handler, color_scheme).await?;
    }

    *redraw_ui = true;
    Ok(())
}

async fn finalize_chat_response(
    app: &mut App<'_>,
    response: CompletionResponse,
    db_handler: &mut ConversationDbHandler<'_>,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    let tokens_predicted =
        response.stats.as_ref().and_then(|s| s.tokens_predicted);

    // stop trying to get more responses
    app.chat_manager.get_active_session().stop_chat_session();

    // finalize with newline for in display
    app.ui
        .response
        .text_append("\n", Some(color_scheme.get_secondary_style()))
        .map_err(|e| ApplicationError::Runtime(e.to_string()))?;

    // add an empty unstyled line
    app.ui
        .response
        .text_append("\n", Some(Style::reset()))
        .map_err(|e| ApplicationError::Runtime(e.to_string()))?;

    // trim exchange + update token length
    app.chat_manager
        .get_active_session()
        .finalize_last_exchange(db_handler, tokens_predicted)
        .await?;

    Ok(())
}
