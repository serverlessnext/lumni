mod clipboard;
mod colorscheme;
mod components;
mod draw;
mod events;
mod modal;
mod ui;
mod widgets;
mod windows;

pub use colorscheme::{ColorScheme, ColorSchemeType};
pub use components::{TextSegment, TextWindowTrait, WindowKind};
pub use draw::draw_ui;
pub use events::{
    CommandLineAction, ConversationEvent, KeyEventHandler, PromptAction,
    WindowEvent,
};
use lumni::api::error::ApplicationError;
pub use modal::{ModalConfigWindow, ModalWindowTrait, ModalWindowType};
pub use ui::TabUi;
pub use windows::{CommandLine, PromptWindow, ResponseWindow};

pub use super::chat::db::{ConversationReader, ModelSpec};
pub use super::chat::{ChatSession, NewConversation};
pub use super::server::{
    ModelServer, ServerManager, ServerTrait, SUPPORTED_MODEL_ENDPOINTS,
};
pub use super::session::TabSession;
pub use crate::external as lumni;
