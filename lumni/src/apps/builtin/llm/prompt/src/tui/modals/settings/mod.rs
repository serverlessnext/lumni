mod creators;
mod list;
mod manager;
mod settings_editor;

use async_trait::async_trait;
use creators::{ProfileCreator, PromptCreator, ProviderCreator};
use crossterm::event::{KeyCode, KeyEvent};
use list::{SettingsList, SettingsListTrait};
use manager::{ConfigItemManager, Creator, CreatorAction, ManagedItem};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs,
};
use ratatui::Frame;
use serde_json::Value as JsonValue;
use settings_editor::{SettingsAction, SettingsEditor, SettingsItem};

use super::widgets::{ListWidget, ListWidgetState, TextArea};
use super::{
    ApplicationError, ChatSessionManager, ConversationDbHandler,
    ConversationEvent, DatabaseConfigurationItem, KeyTrack, MaskMode,
    ModalEvent, ModalWindowTrait, ModalWindowType, ModelServer, ModelSpec,
    ReadDocument, ReadWriteDocument, ServerTrait, TextLine, UserProfile,
    UserProfileDbHandler, WindowMode, SUPPORTED_MODEL_ENDPOINTS,
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
pub enum ConfigTab {
    Profiles,
    Providers,
    Prompts,
}

pub struct SettingsModal {
    pub current_tab: ConfigTab,
    pub tab_focus: TabFocus,
    pub manager: ConfigItemManager,
}

impl SettingsModal {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            current_tab: ConfigTab::Profiles,
            tab_focus: TabFocus::List,
            manager: ConfigItemManager::new(db_handler).await?,
        })
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
    ) -> Result<WindowMode, ApplicationError> {
        if matches!(self.tab_focus, TabFocus::List | TabFocus::Settings) {
            match key_event.code {
                KeyCode::Tab => {
                    self.switch_tab().await?;
                    return Ok(WindowMode::Modal(ModalEvent::UpdateUI));
                }
                KeyCode::Esc | KeyCode::Backspace => {
                    if self.tab_focus == TabFocus::Settings {
                        self.tab_focus = TabFocus::List;
                        return Ok(WindowMode::Modal(ModalEvent::UpdateUI));
                    } else if !self.manager.get_rename_buffer().is_some() {
                        return Ok(WindowMode::Conversation(Some(
                            ConversationEvent::PromptRead,
                        )));
                    }
                }
                _ => {}
            }
        }

        self.manager
            .handle_key_event(key_event, &mut self.tab_focus, self.current_tab)
            .await
    }

    async fn switch_tab(&mut self) -> Result<(), ApplicationError> {
        self.current_tab = match self.current_tab {
            ConfigTab::Profiles => ConfigTab::Providers,
            ConfigTab::Providers => ConfigTab::Prompts,
            ConfigTab::Prompts => ConfigTab::Profiles,
        };
        self.tab_focus = TabFocus::List;
        self.manager.refresh_list(self.current_tab).await?;
        Ok(())
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let tabs = vec!["Profiles", "Providers", "Prompts"];
        let tab_index = match self.current_tab {
            ConfigTab::Profiles => 0,
            ConfigTab::Providers => 1,
            ConfigTab::Prompts => 2,
        };
        let tabs = Tabs::new(tabs)
            .block(Block::default().borders(Borders::ALL))
            .select(tab_index)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(tabs, area);
    }

    fn render_instructions(&self, f: &mut Frame, area: Rect) {
        let instructions = match (self.current_tab, self.tab_focus) {
            (_, TabFocus::List) => {
                "↑↓: Navigate | Enter: Select | R: Rename | D: Delete | Space: \
                 Set Default | Tab: Switch Tab"
            }
            (_, TabFocus::Settings) => {
                let settings_editor = &self.manager.settings_editor;
                match settings_editor.edit_mode {
                    EditMode::NotEditing => {
                        "↑↓: Navigate | Enter: Edit | n: New | N: New Secure | \
                         D: Delete | C: Clear | S: Show/Hide Secure | \
                         ←/Tab/q/Esc: Back to List"
                    }
                    EditMode::EditingValue => "Enter: Save | Esc: Cancel",
                    EditMode::AddingNewKey => {
                        "Enter: Confirm Key | Esc: Cancel"
                    }
                    EditMode::AddingNewValue => {
                        "Enter: Save New Value | Esc: Cancel"
                    }
                }
            }
            (_, TabFocus::Creation) => "Enter: Create | Esc: Cancel",
        };

        let paragraph = Paragraph::new(instructions)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));

        f.render_widget(paragraph, area);
    }
}

#[async_trait]
impl ModalWindowTrait for SettingsModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::ProfileEdit
    }

    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(1),    // Main content
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        self.render_tab_bar(frame, chunks[0]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            .split(chunks[1]);

        self.manager
            .render_list(frame, main_chunks[0], self.tab_focus);
        self.manager
            .render_content(frame, main_chunks[1], self.tab_focus);

        self.render_instructions(frame, chunks[2]);
    }

    async fn poll_background_task(
        &mut self,
    ) -> Result<WindowMode, ApplicationError> {
        self.manager
            .poll_background_task(&mut self.tab_focus, self.current_tab)
            .await
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _chat_manager: &mut ChatSessionManager,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError> {
        self.handle_key_event(key_event.current_key()).await
    }
}

#[derive(Clone, Debug)]
pub enum ConfigItem {
    UserProfile(UserProfile),
    DatabaseConfig(DatabaseConfigurationItem),
}

impl SettingsItem for ConfigItem {
    fn name(&self) -> &str {
        match self {
            ConfigItem::UserProfile(profile) => &profile.name,
            ConfigItem::DatabaseConfig(config) => &config.name,
        }
    }

    fn item_type(&self) -> &'static str {
        match self {
            ConfigItem::UserProfile(_) => "Profile",
            ConfigItem::DatabaseConfig(config) => match config.section.as_str()
            {
                "provider" => "Provider",
                "configuration" => "Configuration",
                _ => "Unknown",
            },
        }
    }
}

#[async_trait]
impl ManagedItem for ConfigItem {
    async fn delete(
        &self,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        match self {
            ConfigItem::UserProfile(profile) => {
                profile.delete(db_handler).await
            }
            ConfigItem::DatabaseConfig(config) => {
                config.delete(db_handler).await
            }
        }
    }

    async fn get_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        match self {
            ConfigItem::UserProfile(profile) => {
                profile.get_settings(db_handler, mask_mode).await
            }
            ConfigItem::DatabaseConfig(config) => {
                config.get_settings(db_handler, mask_mode).await
            }
        }
    }

    async fn update_settings(
        &self,
        db_handler: &mut UserProfileDbHandler,
        settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        match self {
            ConfigItem::UserProfile(profile) => {
                db_handler
                    .update_configuration_item(&profile.into(), settings)
                    .await
            }
            ConfigItem::DatabaseConfig(config) => {
                db_handler
                    .update_configuration_item(config.into(), settings)
                    .await
            }
        }
    }
}

pub struct ConfigItemCreator {
    creator: Box<dyn Creator<ConfigItem>>,
}

#[async_trait]
impl Creator<ConfigItem> for ConfigItemCreator {
    async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        self.creator.handle_input(input).await
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        self.creator.render(f, area);
    }

    async fn create_item(
        &mut self,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        self.creator.create_item().await
    }

    fn poll_background_task(&mut self) -> Option<CreatorAction<ConfigItem>> {
        self.creator.poll_background_task()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ConfigCreationType {
    UserProfile,
    Provider,
    Prompt,
}

impl ConfigItemCreator {
    async fn new(
        db_handler: UserProfileDbHandler,
        creation_type: ConfigCreationType,
    ) -> Result<Self, ApplicationError> {
        let creator: Box<dyn Creator<ConfigItem>> = match creation_type {
            ConfigCreationType::UserProfile => {
                Box::new(ProfileCreator::new(db_handler).await?)
            }
            ConfigCreationType::Provider => {
                Box::new(ProviderCreator::new(db_handler).await?)
            }
            ConfigCreationType::Prompt => {
                Box::new(PromptCreator::new(db_handler))
            }
        };
        Ok(Self { creator })
    }
}

#[async_trait]
impl Creator<ConfigItem> for PromptCreator {
    async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        self.handle_key_event(input).await
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        self.render_creator(f, area);
    }

    async fn create_item(
        &mut self,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match self.create_prompt().await {
            Ok(config_item) => Ok(CreatorAction::Finish(config_item)),
            Err(e) => {
                log::error!("Failed to create prompt: {}", e);
                Ok(CreatorAction::Continue)
            }
        }
    }

    fn poll_background_task(&mut self) -> Option<CreatorAction<ConfigItem>> {
        None // PromptCreator doesn't have a background task
    }
}
