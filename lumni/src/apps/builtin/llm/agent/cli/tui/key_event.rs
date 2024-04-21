
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossterm::event::{
    KeyCode, KeyEvent,
};
use tokio::sync::mpsc;
use tui_textarea::TextArea;
use bytes::Bytes;

use super::command_line::{transition_command_line, CommandLine};
use super::response_window::PromptLogWindow;
use super::editor_window::{PromptAction, TransitionAction};
use super::{TextAreaHandler, MoveCursor};


pub async fn process_key_event(
    key_event: KeyEvent,
    current_mode: TransitionAction,
    command_line_handler: &mut CommandLine,
    command_line: &mut TextArea<'_>,
    editor_window: &mut TextAreaHandler,
    is_running: Arc<AtomicBool>,
    tx: mpsc::Sender<Bytes>,
    response_window: &mut PromptLogWindow<'_>,
) -> TransitionAction {
    match current_mode {
        TransitionAction::CommandLine => {
            // currently in command line mode
            match transition_command_line(
                command_line_handler,
                command_line,
                editor_window.ta_prompt_edit(),
                response_window,
                key_event.into(),
            )
            .await
            {
                TransitionAction::Quit => TransitionAction::Quit,
                TransitionAction::PromptWindow => TransitionAction::PromptWindow,
                TransitionAction::ResponseWindow => TransitionAction::ResponseWindow,
                TransitionAction::Prompt(prompt_action) => {
                    let chat_session = response_window.chat_session();
                    match prompt_action {
                        PromptAction::Write(prompt) => {
                            // Initiate streaming if not already active
                            if !is_running.load(Ordering::SeqCst) {
                                is_running.store(true, Ordering::SeqCst);
                                chat_session
                                    .message(
                                        tx.clone(),
                                        is_running.clone(),
                                        prompt,
                                    )
                                    .await;
                            }
                        }
                        PromptAction::Clear => {
                            chat_session.reset();
                        }
                    }
                    TransitionAction::PromptWindow // Switch to prompt window
                }
                _ => TransitionAction::CommandLine, // Stay in command line mode
            }
        },
        TransitionAction::ResponseWindow => {
            match key_event.code {
                KeyCode::Char(':') => {
                    command_line.insert_str(":");
                    return TransitionAction::CommandLine;
                }
                KeyCode::Right => { response_window.move_cursor(MoveCursor::Right);}
                KeyCode::Left => { response_window.move_cursor(MoveCursor::Left);}
                KeyCode::Up => { response_window.move_cursor(MoveCursor::Up);}
                KeyCode::Down => { response_window.move_cursor(MoveCursor::Down);}
                _ => {}
            };
            TransitionAction::ResponseWindow
        },
        _ => {
            // editor mode
            match editor_window.transition(&key_event.into()).await {
                TransitionAction::Quit => TransitionAction::Quit,
                TransitionAction::CommandLine => {
                    command_line.insert_str(":");
                    TransitionAction::CommandLine
                }
                TransitionAction::Prompt(prompt_action) => {
                    let chat_session = response_window.chat_session();
                    match prompt_action {
                        PromptAction::Write(prompt) => {
                            // Initiate streaming if not already active
                            if !is_running.load(Ordering::SeqCst) {
                                is_running.store(true, Ordering::SeqCst);
                                chat_session
                                    .message(
                                        tx.clone(),
                                        is_running.clone(),
                                        prompt,
                                    )
                                    .await;
                            }
                        }
                        PromptAction::Clear => {
                            chat_session.reset();
                        }
                    }
                    TransitionAction::PromptWindow // Switch to prompt window
                }
                _ => TransitionAction::PromptWindow
            }
        }
    }
}