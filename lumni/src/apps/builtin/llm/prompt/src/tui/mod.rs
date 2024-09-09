mod clipboard;
mod colorscheme;
mod conversations;
mod draw;
mod events;
mod modals;
mod ui;
pub mod widgets;
mod window;
mod workspaces;

pub use colorscheme::{ColorScheme, ColorSchemeType};
pub use conversations::Conversations;
pub use draw::draw_ui;
pub use events::{
    CommandLineAction, ConversationEvent, ConversationSelectEvent,
    FileBrowserEvent, KeyEventHandler, KeyTrack, ModalEvent, PromptAction,
    UserEvent, WindowMode,
};
use lumni::api::error::ApplicationError;
pub use modals::{ModalWindowTrait, ModalWindowType};
pub use ui::{AppUi, ContentDisplayMode, ConversationUi};
pub use window::{
    CommandLine, PromptWindow, ReadDocument, ReadWriteDocument, ResponseWindow,
    SimpleString, TextBuffer, TextDocumentTrait, TextLine, TextSegment,
    TextWindowTrait, WindowKind,
};
pub use workspaces::{Workspace, Workspaces};

use super::chat::db::{
    Conversation, ConversationDatabase, ConversationDbHandler, ConversationId,
    ConversationStatus, MaskMode, ModelSpec, ProviderConfig,
    ProviderConfigOptions, UserProfile, UserProfileDbHandler, WorkspaceId,
};
use super::chat::{
    App, ChatSessionManager, NewConversation, PromptInstruction,
    ThreadedChatSession,
};
use super::server::{ModelServer, ServerTrait, SUPPORTED_MODEL_ENDPOINTS};
use crate::external as lumni;
