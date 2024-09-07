mod list;
mod manager;
mod profile;
mod provider;
mod settings_editor;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use list::{ListItemTrait, SettingsList, SettingsListTrait};
use manager::{Creator, CreatorAction, SettingsManager};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs,
};
use ratatui::Frame;
use settings_editor::{SettingsAction, SettingsEditor};

use super::widgets::{ListWidget, ListWidgetState, TextArea};
use super::{
    ApplicationError, ConversationDbHandler, ConversationEvent, KeyTrack,
    MaskMode, ModalEvent, ModalWindowTrait, ModalWindowType, ModelServer,
    ModelSpec, ProviderConfig, ProviderConfigOptions, ReadDocument,
    ServerTrait, SimpleString, TextLine, TextSegment, ThreadedChatSession,
    UserProfile, UserProfileDbHandler, WindowMode, SUPPORTED_MODEL_ENDPOINTS,
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

pub enum SettingsManagerEnum {
    Profile(SettingsManager<UserProfile>),
    Provider(SettingsManager<ProviderConfig>),
}

impl SettingsManagerEnum {
    fn get_selected_item(&self) -> Option<&dyn SettingsItem> {
        match self {
            SettingsManagerEnum::Profile(manager) => manager
                .list
                .get_selected_item()
                .map(|item| item as &dyn SettingsItem),
            SettingsManagerEnum::Provider(manager) => manager
                .list
                .get_selected_item()
                .map(|item| item as &dyn SettingsItem),
        }
    }

    fn get_settings_editor(&self) -> &SettingsEditor {
        match self {
            SettingsManagerEnum::Profile(manager) => &manager.settings_editor,
            SettingsManagerEnum::Provider(manager) => &manager.settings_editor,
        }
    }

    fn get_rename_buffer(&self) -> Option<&String> {
        match self {
            SettingsManagerEnum::Profile(manager) => {
                manager.rename_buffer.as_ref()
            }
            SettingsManagerEnum::Provider(manager) => {
                manager.rename_buffer.as_ref()
            }
        }
    }
}

pub struct SettingsModal {
    pub current_tab: EditTab,
    pub tab_focus: TabFocus,
    pub manager: SettingsManagerEnum,
}

impl SettingsModal {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            current_tab: EditTab::Profiles,
            tab_focus: TabFocus::List,
            manager: SettingsManagerEnum::Profile(
                SettingsManager::new(db_handler).await?,
            ),
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
                            ConversationEvent::Prompt,
                        )));
                    }
                }
                _ => {}
            }
        }

        match &mut self.manager {
            SettingsManagerEnum::Profile(manager) => {
                manager
                    .handle_key_event(key_event, &mut self.tab_focus)
                    .await
            }
            SettingsManagerEnum::Provider(manager) => {
                manager
                    .handle_key_event(key_event, &mut self.tab_focus)
                    .await
            }
        }
    }

    async fn switch_tab(&mut self) -> Result<(), ApplicationError> {
        let db_handler = self.get_db_handler().clone();
        self.manager = match self.current_tab {
            EditTab::Profiles => {
                self.current_tab = EditTab::Providers;
                SettingsManagerEnum::Provider(
                    SettingsManager::new(db_handler).await?,
                )
            }
            EditTab::Providers => {
                self.current_tab = EditTab::Profiles;
                SettingsManagerEnum::Profile(
                    SettingsManager::new(db_handler).await?,
                )
            }
        };
        self.tab_focus = TabFocus::List;
        self.refresh_list().await?;
        Ok(())
    }

    fn get_db_handler(&self) -> &UserProfileDbHandler {
        match &self.manager {
            SettingsManagerEnum::Profile(m) => &m.db_handler,
            SettingsManagerEnum::Provider(m) => &m.db_handler,
        }
    }

    pub async fn refresh_list(
        &mut self,
    ) -> Result<WindowMode, ApplicationError> {
        match &mut self.manager {
            SettingsManagerEnum::Profile(manager) => {
                manager.refresh_list().await
            }
            SettingsManagerEnum::Provider(manager) => {
                manager.refresh_list().await
            }
        }
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let tabs = vec!["Profiles", "Providers"];
        let tab_index = match self.current_tab {
            EditTab::Profiles => 0,
            EditTab::Providers => 1,
        };
        let tabs = Tabs::new(tabs)
            .block(Block::default().borders(Borders::ALL))
            .select(tab_index)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::Yellow));
        f.render_widget(tabs, area);
    }

    fn render_list(&self, f: &mut Frame, area: Rect) {
        let items = match &self.manager {
            SettingsManagerEnum::Profile(manager) => {
                self.render_list_items(&manager.list)
            }
            SettingsManagerEnum::Provider(manager) => {
                self.render_list_items(&manager.list)
            }
        };

        let title = match self.current_tab {
            EditTab::Profiles => "Profiles",
            EditTab::Providers => "Providers",
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(self.get_selected_index()));

        f.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_list_items<T: ListItemTrait + SettingsItem>(
        &self,
        list: &SettingsList<T>,
    ) -> Vec<ListItem> {
        list.get_items()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let content = if i == list.get_selected_index()
                    && self.manager.get_rename_buffer().is_some()
                {
                    self.manager.get_rename_buffer().unwrap().clone()
                } else {
                    item.to_string()
                };

                let style = if i == list.get_selected_index() {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else if i == list.get_items().len() - 1 {
                    Style::default().fg(Color::Green)
                } else if item.ends_with("(default)") {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                ListItem::new(Span::styled(content, style))
            })
            .collect()
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        match self.tab_focus {
            TabFocus::Settings | TabFocus::List => {
                self.render_settings(f, area);
            }
            TabFocus::Creation => match &mut self.manager {
                SettingsManagerEnum::Profile(manager) => {
                    if let Some(creator) = &mut manager.creator {
                        creator.render(f, area);
                    }
                }
                SettingsManagerEnum::Provider(manager) => {
                    if let Some(creator) = &mut manager.creator {
                        creator.render(f, area);
                    }
                }
            },
        }
    }

    fn render_settings(&self, f: &mut Frame, area: Rect) {
        let item = self.manager.get_selected_item();
        let settings_editor = self.manager.get_settings_editor();

        if let Some(item) = item {
            let settings = settings_editor.get_settings();
            let mut items: Vec<ListItem> = settings
                .as_object()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, (key, value))| {
                    let is_editable = !key.starts_with("__");
                    let display_value =
                        settings_editor.get_display_value(value);

                    let content = if settings_editor.edit_mode
                        == EditMode::EditingValue
                        && i == settings_editor.get_current_field()
                        && is_editable
                    {
                        format!(
                            "{}: {}",
                            key,
                            settings_editor.get_edit_buffer()
                        )
                    } else {
                        format!("{}: {}", key, display_value)
                    };

                    let style = if i == settings_editor.get_current_field() {
                        Style::default()
                            .bg(Color::Rgb(40, 40, 40))
                            .fg(Color::White)
                    } else if is_editable {
                        Style::default().bg(Color::Black).fg(Color::Cyan)
                    } else {
                        Style::default().bg(Color::Black).fg(Color::DarkGray)
                    };
                    ListItem::new(Line::from(vec![Span::styled(
                        content, style,
                    )]))
                })
                .collect();

            // Add new key input field if in AddingNewKey mode
            if settings_editor.edit_mode == EditMode::AddingNewKey {
                let secure_indicator = if settings_editor.is_new_value_secure()
                {
                    "🔒 "
                } else {
                    ""
                };
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!(
                        "{}New key: {}",
                        secure_indicator,
                        settings_editor.get_new_key_buffer()
                    ),
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                )])));
            }

            // Add new value input field if in AddingNewValue mode
            if settings_editor.edit_mode == EditMode::AddingNewValue {
                let secure_indicator = if settings_editor.is_new_value_secure()
                {
                    "🔒 "
                } else {
                    ""
                };
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!(
                        "{}{}: {}",
                        secure_indicator,
                        settings_editor.get_new_key_buffer(),
                        settings_editor.get_edit_buffer()
                    ),
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                )])));
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "{} Settings: {}",
                    item.item_type(),
                    item.name()
                )))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");

            let mut state = ListState::default();
            state.select(Some(settings_editor.get_current_field()));

            f.render_stateful_widget(list, area, &mut state);
        } else {
            let paragraph = Paragraph::new("No item selected").block(
                Block::default().borders(Borders::ALL).title("Settings"),
            );
            f.render_widget(paragraph, area);
        }
    }

    fn render_instructions(&self, f: &mut Frame, area: Rect) {
        let instructions = match (self.current_tab, self.tab_focus) {
            (EditTab::Profiles, TabFocus::List) => {
                "↑↓: Navigate | Enter: Select | R: Rename | D: Delete | Space: \
                 Set Default | Tab: Switch Tab"
            }
            (EditTab::Providers, TabFocus::List) => {
                "↑↓: Navigate | Enter: Select | R: Rename | D: Delete | Tab: \
                 Switch Tab"
            }
            (_, TabFocus::Settings) => {
                let settings_editor = match &self.manager {
                    SettingsManagerEnum::Profile(m) => &m.settings_editor,
                    SettingsManagerEnum::Provider(m) => &m.settings_editor,
                };
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

        let simple_string = SimpleString::from(instructions);
        let wrapped_spans = simple_string.wrapped_spans(
            area.width as usize - 2, // Subtract 2 for left and right borders
            Some(Style::default().fg(Color::Cyan)),
            Some(" | "),
        );

        let wrapped_text: Vec<Line> =
            wrapped_spans.into_iter().map(Line::from).collect();

        let paragraph = Paragraph::new(wrapped_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));

        f.render_widget(paragraph, area);
    }

    fn get_selected_index(&self) -> usize {
        match &self.manager {
            SettingsManagerEnum::Profile(manager) => {
                manager.list.get_selected_index()
            }
            SettingsManagerEnum::Provider(manager) => {
                manager.list.get_selected_index()
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

        self.render_list(frame, main_chunks[0]);
        self.render_content(frame, main_chunks[1]);

        self.render_instructions(frame, chunks[2]);
    }

    async fn poll_background_task(
        &mut self,
    ) -> Result<WindowMode, ApplicationError> {
        match &mut self.manager {
            SettingsManagerEnum::Profile(manager) => {
                if let TabFocus::Creation = self.tab_focus {
                    if let Some(creator) = &mut manager.creator {
                        if let Some(action) = creator.poll_background_task() {
                            match action {
                                CreatorAction::Finish(new_profile) => {
                                    manager.list.add_item(new_profile);
                                    manager.creator = None;
                                    self.tab_focus = TabFocus::List;
                                    self.refresh_list().await?;
                                    return Ok(WindowMode::Modal(
                                        ModalEvent::PollBackGroundTask,
                                    ));
                                }
                                CreatorAction::CreateItem => {
                                    return Ok(WindowMode::Modal(
                                        ModalEvent::PollBackGroundTask,
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            SettingsManagerEnum::Provider(_) => {
                // Provider creation is instant and does not have background tasks
            }
        }
        Ok(WindowMode::Modal(ModalEvent::UpdateUI))
    }

    async fn handle_key_event<'b>(
        &'b mut self,
        key_event: &'b mut KeyTrack,
        _tab_chat: Option<&'b mut ThreadedChatSession>,
        _handler: &mut ConversationDbHandler,
    ) -> Result<WindowMode, ApplicationError> {
        self.handle_key_event(key_event.current_key()).await
    }
}

pub trait SettingsItem {
    fn name(&self) -> &str;
    fn item_type(&self) -> &'static str;
}

impl SettingsItem for UserProfile {
    fn name(&self) -> &str {
        &self.name
    }

    fn item_type(&self) -> &'static str {
        "Profile"
    }
}

impl SettingsItem for ProviderConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn item_type(&self) -> &'static str {
        "Provider"
    }
}
