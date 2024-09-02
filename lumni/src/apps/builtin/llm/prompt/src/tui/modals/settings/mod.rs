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

use super::widgets::{TextAreaState, TextAreaWidget};
use super::{
    ApplicationError, ConversationDbHandler, KeyTrack, MaskMode, ModalAction,
    ModalWindowTrait, ModalWindowType, ModelServer, ModelSpec, ProviderConfig,
    ProviderConfigOptions, ServerTrait, SimpleString, TextLine,
    ThreadedChatSession, UserProfile, UserProfileDbHandler, WindowEvent,
    SUPPORTED_MODEL_ENDPOINTS,
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
        // Handle common cases for List and Settings tab focus
        if matches!(self.tab_focus, TabFocus::List | TabFocus::Settings) {
            match key_event.code {
                KeyCode::Tab => {
                    self.switch_tab().await?;
                    return Ok(WindowEvent::Modal(ModalAction::UpdateUI));
                }
                KeyCode::Esc | KeyCode::Backspace => {
                    if self.tab_focus == TabFocus::Settings {
                        self.tab_focus = TabFocus::List;
                        return Ok(WindowEvent::Modal(ModalAction::UpdateUI));
                    } else {
                        return Ok(WindowEvent::PromptWindow(None));
                    }
                }
                _ => {}
            }
        }

        // Pass all other key events to the respective manager
        match self.current_tab {
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
            TabFocus::Creation => match modal.current_tab {
                EditTab::Profiles => {
                    if let Some(creator) = &modal.profile_manager.creator {
                        creator.render(f, area);
                    }
                }
                EditTab::Providers => {
                    if let Some(creator) = &modal.provider_manager.creator {
                        creator.render(f, area);
                    }
                }
            },
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

    async fn poll_background_task(
        &mut self,
    ) -> Result<WindowEvent, ApplicationError> {
        match self.current_tab {
            EditTab::Profiles => {
                if let TabFocus::Creation = self.tab_focus {
                    if let Some(creator) = &mut self.profile_manager.creator {
                        if let Some(action) = creator.poll_background_task() {
                            match action {
                                CreatorAction::Finish(new_profile) => {
                                    self.profile_manager
                                        .list
                                        .add_item(new_profile);
                                    self.profile_manager.creator = None;
                                    self.tab_focus = TabFocus::List;
                                    return Ok(WindowEvent::Modal(
                                        ModalAction::PollBackGroundTask,
                                    ));
                                }
                                CreatorAction::CreateItem => {
                                    return Ok(WindowEvent::Modal(
                                        ModalAction::PollBackGroundTask,
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            EditTab::Providers => {
                // Provider creation is instant and does not have background tasks
            }
        }
        Ok(WindowEvent::Modal(ModalAction::UpdateUI))
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: Option<&'b mut ThreadedChatSession>,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowEvent, ApplicationError> {
        self.handle_key_event(key_event.current_key().clone()).await
    }
}
