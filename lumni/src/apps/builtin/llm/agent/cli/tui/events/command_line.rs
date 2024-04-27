use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use super::key_event::KeyTrack;
use super::{
    transition_command_line, ChatSession, CommandLine, PromptAction,
    PromptWindow, ResponseWindow, WindowEvent,
};

pub async fn handle_command_line_event(
    key_track: &KeyTrack,
    command_line_handler: &mut CommandLine,
    response_window: &mut ResponseWindow<'_>,
    chat_session: &mut ChatSession,
    prompt_window: &mut PromptWindow<'_>,
    command_line: &mut TextArea<'_>,
    tx: mpsc::Sender<Bytes>,
    is_running: Arc<AtomicBool>,
) -> WindowEvent {
    let key_event = key_track.current_key();
    match transition_command_line(
        command_line_handler,
        command_line,
        prompt_window,
        response_window,
        key_event.into(),
    )
    .await
    {
        WindowEvent::Quit => WindowEvent::Quit,
        WindowEvent::PromptWindow => WindowEvent::PromptWindow,
        WindowEvent::ResponseWindow => WindowEvent::ResponseWindow,
        WindowEvent::Prompt(prompt_action) => {
            match prompt_action {
                PromptAction::Write(prompt) => {
                    send_prompt(chat_session, tx, is_running, prompt).await;
                }
                PromptAction::Clear => {
                    chat_session.reset();
                }
            }
            WindowEvent::PromptWindow // Switch to prompt window
        }
        _ => WindowEvent::CommandLine, // Stay in command line mode
    }
}

pub async fn send_prompt(
    chat_session: &mut ChatSession,
    tx: mpsc::Sender<Bytes>,
    is_running: Arc<AtomicBool>,
    prompt: String,
) {
    if !is_running.load(Ordering::SeqCst) {
        is_running.store(true, Ordering::SeqCst);
        chat_session
            .message(tx.clone(), is_running.clone(), prompt)
            .await;
    }
}
