use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossterm::event::{
    poll, read, Event, KeyEvent, MouseEvent, MouseEventKind,
};
use futures::future::FutureExt;
use lumni::api::error::ApplicationError;
use ratatui::backend::Backend;
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::time::{interval, Duration};

use super::chat_session_manager::ChatEvent;
use super::db::{ConversationDatabase, ConversationDbHandler};
use super::{
    App, ColorScheme, CommandLineAction, ConversationEvent, KeyEventHandler,
    ModalWindowType, PromptAction, PromptInstruction, TextWindowTrait,
    WindowEvent, WindowKind,
};
pub use crate::external as lumni;

pub async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App<'_>,
    db_conn: Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
    let color_scheme = app.color_scheme.clone();
    let mut tick = interval(Duration::from_millis(16)); // ~60 fps
    let keep_running = Arc::new(AtomicBool::new(true));
    let mut current_mode = Some(WindowEvent::ResponseWindow);
    let mut key_event_handler = KeyEventHandler::new();
    let mut redraw_ui = true;
    let conversation_id = app.get_conversation_id_for_active_session().clone();
    let mut db_handler =
        db_conn.get_conversation_handler(Some(conversation_id));

    loop {
        tokio::select! {
            _ = tick.tick().fuse() => {
                handle_tick(terminal, &mut app, &mut redraw_ui, &mut current_mode, &mut key_event_handler, &keep_running, &mut db_handler, &color_scheme).await?;
            }
            _ = async {
                // Process chat events
                let events = app.chat_manager.get_active_session().subscribe().recv().await;
                if let Ok(event) = events {
                    match event {
                        ChatEvent::ResponseUpdate(content) => {
                            app.ui.response.text_append(&content, Some(color_scheme.get_secondary_style()))?;
                            redraw_ui = true;
                        },
                        ChatEvent::FinalResponse => {
                            app.ui.response.text_append("\n\n", Some(color_scheme.get_secondary_style()))?;
                            redraw_ui = true;
                        },
                        ChatEvent::Error(error) => {
                            log::error!("Chat session error: {}", error);
                            redraw_ui = true;
                        }
                    }
                }
                Ok::<(), ApplicationError>(())
            }.fuse() => {}
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
    db_handler: &mut ConversationDbHandler,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    if *redraw_ui {
        app.draw_ui(terminal).await?;
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
    db_handler: &mut ConversationDbHandler,
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
            handle_prompt_action(app, prompt_action.clone(), color_scheme)
                .await?;
            *current_mode = Some(app.ui.set_prompt_window(false));
        }
        Some(WindowEvent::CommandLine(action)) => {
            handle_command_line_action(app, action.clone());
        }
        Some(WindowEvent::Modal(modal_window_type)) => {
            handle_modal_window(app, modal_window_type, db_handler).await?;
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
) -> Result<(), ApplicationError> {
    match prompt_action {
        PromptAction::Write(prompt) => {
            send_prompt(app, &prompt, color_scheme).await?;
        }
        PromptAction::Stop => {
            app.stop_active_chat_session().await;
            app.ui
                .response
                .text_append("\n", Some(color_scheme.get_secondary_style()))?;
            // add an empty unstyled line
            app.ui.response.text_append("\n", Some(Style::reset()))?;
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

async fn handle_modal_window(
    app: &mut App<'_>,
    modal_window_type: &ModalWindowType,
    handler: &ConversationDbHandler,
) -> Result<(), ApplicationError> {
    // switch to, or stay in modal window
    match modal_window_type {
        ModalWindowType::ConversationList(Some(_)) => {
            // reload chat on any conversation event
            _ = app.reload_conversation().await;
            return Ok(());
        }
        _ => {}
    }

    if app.ui.needs_modal_update(modal_window_type) {
        app.ui
            .set_new_modal(modal_window_type.clone(), handler)
            .await?;
    }
    Ok(())
}

async fn handle_prompt_window_event(
    app: &mut App<'_>,
    event: Option<ConversationEvent>,
    db_handler: &mut ConversationDbHandler,
) -> Result<(), ApplicationError> {
    // switch to prompt window
    match event {
        Some(ConversationEvent::NewConversation(new_conversation)) => {
            // TODO: handle in modal
            let prompt_instruction =
                PromptInstruction::new(new_conversation.clone(), db_handler)
                    .await?;
            _ = app
                .load_instruction_for_active_session(prompt_instruction)
                .await?;
            _ = app.reload_conversation().await;
        }
        Some(_) => {
            // any other ConversationEvent is a reload
            _ = app.reload_conversation().await;
        }
        None => {
            log::debug!("No prompt window event to handle");
        }
    }
    // ensure modal is closed
    app.ui.clear_modal();
    Ok(())
}

async fn send_prompt<'a>(
    app: &mut App<'a>,
    prompt: &str,
    color_scheme: &ColorScheme,
) -> Result<(), ApplicationError> {
    // prompt should end with single newline
    let formatted_prompt = format!("{}\n", prompt.trim_end());
    let result = app
        .chat_manager
        .get_active_session()
        .message(&formatted_prompt)
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
