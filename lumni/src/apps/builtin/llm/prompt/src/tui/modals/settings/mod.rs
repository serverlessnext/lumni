mod profile;
mod provider;
mod settings_editor;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use profile::{ProfileEditRenderer, ProfileManager};
use provider::ProviderManager;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use ratatui::Frame;
pub use settings_editor::{SettingsAction, SettingsEditor};

use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, MaskMode, ModalAction,
    ModalWindowTrait, ModalWindowType, ModelServer, ModelSpec, ProviderConfig,
    ProviderConfigOptions, ServerTrait, SimpleString, ThreadedChatSession,
    UserProfile, UserProfileDbHandler, WindowEvent, SUPPORTED_MODEL_ENDPOINTS,
};

#[derive(Debug)]
pub enum BackgroundTaskResult {
    ProfileCreated(Result<UserProfile, ApplicationError>),
}
pub trait GenericList {
    fn get_items(&self) -> Vec<String>;
    fn get_selected_index(&self) -> usize;
}

pub trait Creator {
    fn render(&self, f: &mut Frame, area: Rect);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    NotEditing,
    EditingValue,
    AddingNewKey,
    AddingNewValue,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabFocus {
    List,
    Settings,
    Creation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditTab {
    Profiles,
    Providers,
}
pub struct SettingsModal {
    pub current_tab: EditTab,
    pub tab_focus: TabFocus,
    pub profile_manager: ProfileManager,
    pub provider_manager: ProviderManager,
    renderer: ProfileEditRenderer,
}

impl SettingsModal {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let profile_manager = ProfileManager::new(db_handler.clone()).await?;

        Ok(Self {
            current_tab: EditTab::Profiles,
            tab_focus: TabFocus::List,
            profile_manager,
            provider_manager: ProviderManager::new(db_handler.clone()).await?,
            renderer: ProfileEditRenderer::new(),
        })
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
    ) -> Result<WindowEvent, ApplicationError> {
        match key_event.code {
            KeyCode::Tab => {
                self.switch_tab().await?;
                Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
            }
            KeyCode::Esc => {
                if self.tab_focus == TabFocus::Settings {
                    self.tab_focus = TabFocus::List;
                    Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
                } else {
                    Ok(WindowEvent::PromptWindow(None))
                }
            }
            _ => match self.current_tab {
                EditTab::Profiles => {
                    self.profile_manager
                        .handle_key_event(key_event, &mut self.tab_focus)
                        .await
                }
                EditTab::Providers => {
                    self.provider_manager
                        .handle_key_event(key_event, &mut self.tab_focus)
                        .await
                }
            },
        }
    }

    async fn switch_tab(&mut self) -> Result<(), ApplicationError> {
        self.current_tab = match self.current_tab {
            EditTab::Profiles => EditTab::Providers,
            EditTab::Providers => EditTab::Profiles,
        };
        self.tab_focus = TabFocus::List;
        self.refresh_list().await?;
        Ok(())
    }

    pub fn get_current_list(&self) -> &dyn GenericList {
        match self.current_tab {
            EditTab::Profiles => &self.profile_manager.list,
            EditTab::Providers => &self.provider_manager.list,
        }
    }

    pub fn get_current_settings_editor(&self) -> &SettingsEditor {
        match self.current_tab {
            EditTab::Profiles => &self.profile_manager.settings_editor,
            EditTab::Providers => &self.provider_manager.settings_editor,
        }
    }

    pub fn get_current_creator(&self) -> Option<&dyn Creator> {
        match self.current_tab {
            EditTab::Profiles => self
                .profile_manager
                .creator
                .as_ref()
                .map(|c| c as &dyn Creator),
            EditTab::Providers => self
                .provider_manager
                .creator
                .as_ref()
                .map(|c| c as &dyn Creator),
        }
    }

    pub async fn refresh_list(
        &mut self,
    ) -> Result<WindowEvent, ApplicationError> {
        match self.current_tab {
            EditTab::Profiles => {
                self.profile_manager.refresh_profile_list().await
            }
            EditTab::Providers => {
                self.provider_manager.refresh_provider_list().await
            }
        }
    }

    pub fn get_rename_buffer(&self) -> Option<&String> {
        match self.current_tab {
            EditTab::Profiles => self.profile_manager.get_rename_buffer(),
            EditTab::Providers => None, // TODO: Implement
        }
    }
}

#[async_trait]
impl ModalWindowTrait for SettingsModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::ProfileEdit
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);
        match self.current_tab {
            EditTab::Profiles => self.renderer.render_layout(frame, area, self),
            EditTab::Providers => {
                self.renderer.render_layout(frame, area, self)
            }
            // TODO: move render to specific renderer
            //EditTab::Providers => self.provider_manager.render(frame, area),
        }
    }

    async fn refresh(&mut self) -> Result<WindowEvent, ApplicationError> {
        // Runs when a list item is being created or updated in the background
        self.refresh_list().await
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: &'b mut ThreadedChatSession,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError> {
        self.handle_key_event(key_event.current_key().clone()).await
    }
}
