mod clipboard;
mod colorscheme;
mod draw;
mod events;
mod modals;
mod ui;
mod window;

pub use colorscheme::{ColorScheme, ColorSchemeType};
pub use draw::draw_ui;
pub use events::{
    CommandLineAction, ConversationEvent, KeyEventHandler, KeyTrack,
    PromptAction, WindowEvent,
};
use lumni::api::error::ApplicationError;
pub use modals::{ModalWindowTrait, ModalWindowType};
pub use ui::AppUi;
pub use window::{
    CommandLine, PromptWindow, ResponseWindow, Scroller, TextLine, TextSegment,
    TextWindowTrait, WindowKind,
};

use super::chat::db::{
    Conversation, ConversationDatabase, ConversationDbHandler, ConversationId,
    ConversationStatus, MaskMode, UserProfileDbHandler,
};
use super::chat::{
    App, NewConversation, PromptInstruction, ThreadedChatSession,
};
use super::server::{
    ModelServer, ServerManager, ServerTrait, SUPPORTED_MODEL_ENDPOINTS,
};
use crate::external as lumni;
