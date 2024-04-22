use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use crossterm::event::KeyEvent;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use super::{
    transition_command_line, CommandLine, PromptAction, PromptLogWindow,
    TextAreaHandler, WindowEvent,
};

pub async fn handle_command_line_event(
    key_event: KeyEvent,
    command_line_handler: &mut CommandLine,
    response_window: &mut PromptLogWindow<'_>,
    editor_window: &mut TextAreaHandler,
    command_line: &mut TextArea<'_>,
    tx: mpsc::Sender<Bytes>,
    is_running: Arc<AtomicBool>,
) -> WindowEvent {
    match transition_command_line(
        command_line_handler,
        command_line,
        editor_window.ta_prompt_edit(),
        response_window,
        key_event.into(),
    )
    .await
    {
        WindowEvent::Quit => WindowEvent::Quit,
        WindowEvent::PromptWindow => WindowEvent::PromptWindow,
        WindowEvent::ResponseWindow => WindowEvent::ResponseWindow,
        WindowEvent::Prompt(prompt_action) => {
            let chat_session = response_window.chat_session();
            match prompt_action {
                PromptAction::Write(prompt) => {
                    // Initiate streaming if not already active
                    if !is_running.load(Ordering::SeqCst) {
                        is_running.store(true, Ordering::SeqCst);
                        chat_session
                            .message(tx.clone(), is_running.clone(), prompt)
                            .await;
                    }
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
