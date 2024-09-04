mod clipboard;
mod colorscheme;
mod draw;
mod events;
mod modals;
mod ui;
pub mod widgets;
mod window;

pub use colorscheme::{ColorScheme, ColorSchemeType};
pub use draw::draw_ui;
pub use events::{
    CommandLineAction, ConversationEvent, KeyEventHandler, KeyTrack,
    PromptAction, UserEvent, WindowEvent,
};
use lumni::api::error::ApplicationError;
pub use modals::{ModalAction, ModalWindowTrait, ModalWindowType};
pub use ui::AppUi;
pub use window::{
    CommandLine, PromptWindow, ReadDocument, ReadWriteDocument, ResponseWindow,
    SimpleString, TextBuffer, TextDocumentTrait, TextLine, TextSegment,
    TextWindowTrait, WindowKind,
};

use super::chat::db::{
    Conversation, ConversationDatabase, ConversationDbHandler, ConversationId,
    ConversationStatus, MaskMode, ModelSpec, ProviderConfig,
    ProviderConfigOptions, UserProfile, UserProfileDbHandler,
};
use super::chat::{
    App, NewConversation, PromptInstruction, ThreadedChatSession,
};
use super::server::{ModelServer, ServerTrait, SUPPORTED_MODEL_ENDPOINTS};
use crate::external as lumni;
