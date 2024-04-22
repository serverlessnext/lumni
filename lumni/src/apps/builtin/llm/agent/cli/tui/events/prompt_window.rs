use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use bytes::Bytes;
use crossterm::event::KeyEvent;
use tui_textarea::TextArea;

use super::{WindowEvent, PromptLogWindow, PromptAction,
    TextAreaHandler,
};

pub async fn handle_prompt_window_event(
    key_event: KeyEvent,
    response_window: &mut PromptLogWindow<'_>,
    editor_window: &mut TextAreaHandler,
    command_line: &mut TextArea<'_>,
    tx: mpsc::Sender<Bytes>,
    is_running: Arc<AtomicBool>,
) -> WindowEvent {
    match editor_window.transition(&key_event.into()).await {
        WindowEvent::Quit => WindowEvent::Quit,
        WindowEvent::CommandLine => {
            command_line.insert_str(":");
            WindowEvent::CommandLine
        }
        WindowEvent::Prompt(prompt_action) => {
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
            WindowEvent::PromptWindow // Switch to prompt window
        }
        _ => WindowEvent::PromptWindow
    }

}
