mod profile_edit_modal;
mod profile_edit_renderer;
mod profile_list;
mod provider_manager;
mod settings_editor;

pub use profile_edit_modal::ProfileEditModal;
pub use provider_manager::{
    AdditionalSetting, ProviderConfig, ProviderManager,
};

use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, MaskMode, ModalAction,
    ModalWindowTrait, ModalWindowType, ModelIdentifier, ModelServer, ModelSpec,
    ServerTrait, SimpleString, ThreadedChatSession, UserProfile,
    UserProfileDbHandler, WindowEvent, SUPPORTED_MODEL_ENDPOINTS,
};

#[derive(Debug)]
pub enum BackgroundTaskResult {
    ProfileCreated(Result<UserProfile, ApplicationError>),
}
