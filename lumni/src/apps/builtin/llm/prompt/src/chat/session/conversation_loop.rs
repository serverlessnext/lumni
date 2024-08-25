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
    App, CommandLineAction, ConversationEvent, KeyEventHandler, ModalAction,
    PromptAction, PromptInstruction, TextWindowTrait, UserEvent, WindowEvent,
    WindowKind,
};
pub use crate::external as lumni;

pub async fn prompt_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App<'_>,
    mut db_conn: Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
    let color_scheme = app.color_scheme.clone();
    let mut tick = interval(Duration::from_millis(16)); // ~60 fps
    let keep_running = Arc::new(AtomicBool::new(true));

    let mut redraw_ui = true;
    let mut current_mode = Some(WindowEvent::PromptWindow(None));
    let mut key_event_handler = KeyEventHandler::new();

    loop {
        tokio::select! {
            _ = tick.tick().fuse() => {
                handle_input_event(
                    &mut app,
                    &mut redraw_ui,
                    &mut current_mode,
                    &mut key_event_handler,
                    &keep_running,
                    &mut db_conn,
                ).await?;
            }
            _ = async {
                // Process chat events
                let events = app.chat_manager.get_active_session()?.subscribe().recv().await;
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

        // Handle processing state
        if app.is_processing {
            redraw_ui = true;

            // Handle modal refresh if it's responsible for the processing state
            if let Some(WindowEvent::Modal(_)) = current_mode {
                current_mode =
                    Some(handle_modal_refresh(&mut app, &mut db_conn).await?);
            }
        }

        if redraw_ui {
            app.draw_ui(terminal).await?;
            redraw_ui = false;
        }
    }
    Ok(())
}

async fn handle_input_event(
    app: &mut App<'_>,
    redraw_ui: &mut bool,
    current_mode: &mut Option<WindowEvent>,
    key_event_handler: &mut KeyEventHandler,
    keep_running: &Arc<AtomicBool>,
    db_conn: &mut Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
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
                    db_conn,
                )
                .await?;
            }
            Event::Mouse(mouse_event) => {
                if !handle_mouse_event(app, mouse_event) {
                    return Ok(()); // skip redraw_ui if mouse event was not handled
                }
            }
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
    db_conn: &mut Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
    if let Some(mode) = current_mode.take() {
        let mut conversation_handler = db_conn.get_conversation_handler(
            app.get_conversation_id_for_active_session(),
        );
        let new_window_event = key_event_handler
            .process_key(
                key_event,
                &mut app.ui,
                app.chat_manager.get_active_session()?,
                mode,
                keep_running.clone(),
                &mut conversation_handler,
            )
            .await?;
        let result =
            handle_window_event(app, new_window_event, db_conn).await?;
        *current_mode = Some(result);
    }

    Ok(())
}

async fn handle_modal_refresh(
    app: &mut App<'_>,
    db_conn: &mut Arc<ConversationDatabase>,
) -> Result<WindowEvent, ApplicationError> {
    let modal = app
        .ui
        .modal
        .as_mut()
        .expect("Modal should exist when in Modal mode");
    let refresh_result = modal.refresh().await?;
    match refresh_result {
        WindowEvent::Modal(ModalAction::Refresh) => {
            // If the modal still needs refreshing, keep the processing state
            app.is_processing = true;
            Ok(WindowEvent::Modal(ModalAction::Refresh))
        }
        other_event => {
            // Handle the event returned by refresh
            app.is_processing = false;
            handle_window_event(app, other_event, db_conn).await
        }
    }
}

async fn handle_window_event(
    app: &mut App<'_>,
    window_event: WindowEvent,
    db_conn: &mut Arc<ConversationDatabase>,
) -> Result<WindowEvent, ApplicationError> {
    match window_event {
        WindowEvent::Prompt(ref prompt_action) => {
            handle_prompt_action(app, prompt_action.clone()).await?;
            Ok(app.ui.set_prompt_window(false))
        }
        WindowEvent::CommandLine(ref action) => {
            handle_command_line_action(app, action.clone());
            Ok(window_event)
        }
        WindowEvent::Modal(ref action) => match action {
            ModalAction::Refresh => {
                app.is_processing = true;
                Ok(window_event)
            }
            ModalAction::Open(ref modal_window_type) => {
                app.ui
                    .set_new_modal(
                        modal_window_type.clone(),
                        db_conn,
                        app.get_conversation_id_for_active_session(),
                    )
                    .await?;
                // refresh at least once after opening modal
                app.is_processing = true;
                Ok(WindowEvent::Modal(ModalAction::Refresh))
            }
            ModalAction::Event(ref user_event) => {
                handle_modal_user_event(app, user_event, db_conn).await?;
                Ok(window_event)
            }
            _ => Ok(window_event),
        },
        WindowEvent::PromptWindow(ref event) => {
            let mut conversation_handler = db_conn.get_conversation_handler(
                app.get_conversation_id_for_active_session(),
            );
            handle_prompt_window_event(
                app,
                event.clone(),
                &mut conversation_handler,
            )
            .await?;
            Ok(window_event)
        }
        _ => Ok(window_event),
    }
}

fn handle_mouse_event(app: &mut App, mouse_event: MouseEvent) -> bool {
    // handle mouse events in response window
    // return true if event was handled
    let window = &mut app.ui.response;
    match mouse_event.kind {
        MouseEventKind::ScrollUp => window.scroll_up(),
        MouseEventKind::ScrollDown => window.scroll_down(),
        _ => {
            // ignore other mouse events
            return false;
        }
    }
    true // handled mouse event
}

async fn handle_prompt_action(
    app: &mut App<'_>,
    prompt_action: PromptAction,
) -> Result<(), ApplicationError> {
    let color_scheme = app.color_scheme.clone();
    match prompt_action {
        PromptAction::Write(prompt) => {
            send_prompt(app, &prompt).await?;
        }
        PromptAction::Stop => {
            app.stop_active_chat_session().await?;
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

async fn handle_modal_user_event(
    app: &mut App<'_>,
    user_event: &UserEvent,
    _db_conn: &mut Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
    // switch to, or stay in modal window
    match user_event {
        UserEvent::ReloadConversation => {
            _ = app.reload_conversation().await;
            return Ok(());
        }
        _ => {}
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
) -> Result<(), ApplicationError> {
    // prompt should end with single newline
    let color_scheme = app.color_scheme.clone();
    let formatted_prompt = format!("{}\n", prompt.trim_end());
    let result = app
        .chat_manager
        .get_active_session()?
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
        Err(prompt_error) => {
            // show error in alert window
            app.ui.command_line.set_alert(&prompt_error.to_string())?;
        }
    }
    Ok(())
}
