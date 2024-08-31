mod list;
mod manager;
mod profile;
mod provider;
mod renderer;
mod settings_editor;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use list::SettingsListTrait;
use manager::{Creator, CreatorAction, SettingsManager};
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use ratatui::Frame;
use renderer::SettingsRenderer;
use settings_editor::{SettingsAction, SettingsEditor};

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
    pub profile_manager: SettingsManager<UserProfile>,
    pub provider_manager: SettingsManager<ProviderConfig>,
    renderer: SettingsRenderer,
}

impl SettingsModal {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            current_tab: EditTab::Profiles,
            tab_focus: TabFocus::List,
            profile_manager: SettingsManager::new(db_handler.clone()).await?,
            provider_manager: SettingsManager::new(db_handler.clone()).await?,
            renderer: SettingsRenderer::new(),
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

    pub fn get_current_list(&self) -> &dyn SettingsListTrait {
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

    pub async fn refresh_list(
        &mut self,
    ) -> Result<WindowEvent, ApplicationError> {
        match self.current_tab {
            EditTab::Profiles => self.profile_manager.refresh_list().await,
            EditTab::Providers => self.provider_manager.refresh_list().await,
        }
    }

    pub fn get_rename_buffer(&self) -> Option<&String> {
        match self.current_tab {
            EditTab::Profiles => self.profile_manager.get_rename_buffer(),
            EditTab::Providers => self.provider_manager.get_rename_buffer(),
        }
    }

    fn render_settings(&self, f: &mut Frame, area: Rect) {
        match self.current_tab {
            EditTab::Profiles => self.renderer.render_settings(
                f,
                area,
                self.profile_manager.list.get_selected_item(),
                &self.profile_manager.settings_editor,
            ),
            EditTab::Providers => self.renderer.render_settings(
                f,
                area,
                self.provider_manager.list.get_selected_item(),
                &self.provider_manager.settings_editor,
            ),
        }
    }

    fn render_content(f: &mut Frame, area: Rect, modal: &SettingsModal) {
        match modal.tab_focus {
            TabFocus::Settings | TabFocus::List => {
                modal.render_settings(f, area);
            }
            TabFocus::Creation => {
                modal.render_creator(f, area);
            }
        }
    }

    fn render_creator(&self, f: &mut Frame, area: Rect) {
        match self.current_tab {
            EditTab::Profiles => {
                if let Some(creator) = &self.profile_manager.creator {
                    creator.as_ref().render(f, area);
                }
            }
            EditTab::Providers => {
                if let Some(creator) = &self.provider_manager.creator {
                    creator.as_ref().render(f, area);
                }
            }
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
        self.renderer
            .render_layout(frame, area, self, &Self::render_content);
    }

    async fn refresh(&mut self) -> Result<WindowEvent, ApplicationError> {
        Ok(WindowEvent::Modal(ModalAction::WaitForKeyEvent))
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
