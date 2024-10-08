use std::error::Error;
mod completion_options;
mod conversation;
pub mod db;
mod prompt;
mod session;

pub use completion_options::ChatCompletionOptions;
pub use conversation::{ConversationCache, NewConversation, PromptInstruction};
use prompt::Prompt;
pub use prompt::{PromptInstructionBuilder, PromptRole};
pub use session::{
    prompt_app, App, ChatEvent, ChatSessionManager, ThreadedChatSession,
};

pub use super::defaults::*;
pub use super::error::{PromptError, PromptNotReadyReason};
use super::server::{
    CompletionResponse, ModelBackend, ModelServer, ServerManager,
};
use super::tui::{
    draw_ui, AppUi, ColorScheme, ColorSchemeType, CommandLineAction,
    KeyEventHandler, ModalEvent, PromptAction, SimpleString, TextLine,
    TextSegment, TextWindowTrait, UserEvent, WindowKind, WindowMode,
};

// gets PERSONAS from the generated code
include!(concat!(env!("OUT_DIR"), "/llm/prompt/templates.rs"));

// TODO: add ability to change assistant
#[allow(dead_code)]
pub fn list_assistants() -> Result<Vec<String>, Box<dyn Error>> {
    let prompts: Vec<Prompt> = serde_yaml::from_str(PERSONAS)?;
    let assistants: Vec<String> =
        prompts.iter().map(|p| p.name().to_owned()).collect();
    Ok(assistants)
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: PromptRole,
    pub content: String,
}
