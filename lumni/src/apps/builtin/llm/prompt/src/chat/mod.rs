use std::error::Error;
mod completion_options;
mod conversation;
pub mod db;
mod prompt;
mod session;

pub use completion_options::ChatCompletionOptions;
pub use conversation::{ConversationCache, NewConversation, PromptInstruction};
use prompt::Prompt;
pub use prompt::{AssistantManager, PromptRole};
pub use session::{prompt_app, App, ChatEvent, ThreadedChatSession};

pub use super::defaults::*;
pub use super::error::{PromptError, PromptNotReadyReason};
pub use super::server::{CompletionResponse, ModelServer, ServerManager};
use super::tui::{
    draw_ui, AppUi, ColorScheme, ColorSchemeType, CommandLineAction,
    ConversationEvent, KeyEventHandler, ModalWindowType, PromptAction,
    TextLine, TextSegment, TextWindowTrait, WindowEvent, WindowKind,
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
