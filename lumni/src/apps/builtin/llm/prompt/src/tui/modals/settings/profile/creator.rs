use std::time::Instant;

use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use serde_json::json;
use tokio::sync::mpsc;

use super::provider::{ProviderCreator, ProviderCreatorAction};
use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileCreationStep {
    EnterName,
    SelectProvider,
    CreateProvider,
    ConfirmCreate,
    CreatingProfile,
}

#[derive(Debug, Clone)]
pub enum ProfileCreatorAction {
    Refresh,
    WaitForKeyEvent,
    Cancel,
    CreateProfile,
    SwitchToProviderCreation,
    FinishProviderCreation(ProviderConfig),
}

pub struct ProfileCreator {
    pub new_profile_name: String,
    pub creation_step: ProfileCreationStep,
    db_handler: UserProfileDbHandler,
    pub background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    pub task_start_time: Option<Instant>,
    selected_provider: Option<ProviderConfig>,
    provider_configs: Vec<ProviderConfig>,
    selected_provider_index: Option<usize>,
    provider_creator: Option<ProviderCreator>,
}

impl ProfileCreator {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let provider_configs = db_handler.load_provider_configs().await?;

        Ok(Self {
            new_profile_name: String::new(),
            creation_step: ProfileCreationStep::EnterName,
            db_handler,
            background_task: None,
            task_start_time: None,
            selected_provider: None,
            provider_configs,
            selected_provider_index: None,
            provider_creator: None,
        })
    }

    pub async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        match self.creation_step {
            ProfileCreationStep::EnterName => self.handle_enter_name(input),
            ProfileCreationStep::SelectProvider => {
                self.handle_select_provider(input).await
            }
            ProfileCreationStep::CreateProvider => {
                self.handle_create_provider(input).await
            }
            ProfileCreationStep::ConfirmCreate => {
                self.handle_confirm_create(input)
            }
            ProfileCreationStep::CreatingProfile => {
                Ok(ProfileCreatorAction::WaitForKeyEvent)
            }
        }
    }

    fn handle_enter_name(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        match input.code {
            KeyCode::Char(c) => {
                self.new_profile_name.push(c);
                Ok(ProfileCreatorAction::Refresh)
            }
            KeyCode::Backspace => {
                self.new_profile_name.pop();
                Ok(ProfileCreatorAction::Refresh)
            }
            KeyCode::Enter => {
                if !self.new_profile_name.is_empty() {
                    self.creation_step = ProfileCreationStep::SelectProvider;
                    Ok(ProfileCreatorAction::Refresh)
                } else {
                    Ok(ProfileCreatorAction::WaitForKeyEvent)
                }
            }
            KeyCode::Esc => Ok(ProfileCreatorAction::Cancel),
            _ => Ok(ProfileCreatorAction::WaitForKeyEvent),
        }
    }

    async fn handle_select_provider(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        match input.code {
            KeyCode::Up => {
                if let Some(index) = self.selected_provider_index.as_mut() {
                    if *index > 0 {
                        *index -= 1;
                    } else {
                        *index = self.provider_configs.len(); // Wrap to "Create new Provider" option
                    }
                } else {
                    self.selected_provider_index =
                        Some(self.provider_configs.len()); // Select "Create new Provider"
                }
                Ok(ProfileCreatorAction::Refresh)
            }
            KeyCode::Down => {
                if let Some(index) = self.selected_provider_index.as_mut() {
                    if *index < self.provider_configs.len() {
                        *index += 1;
                    } else {
                        *index = 0; // Wrap to first provider
                    }
                } else {
                    self.selected_provider_index = Some(0);
                }
                Ok(ProfileCreatorAction::Refresh)
            }
            KeyCode::Enter => {
                if let Some(index) = self.selected_provider_index {
                    if index == self.provider_configs.len() {
                        // "Create new Provider" option selected
                        self.creation_step =
                            ProfileCreationStep::CreateProvider;
                        self.provider_creator = Some(
                            ProviderCreator::new(self.db_handler.clone())
                                .await?,
                        );
                        Ok(ProfileCreatorAction::SwitchToProviderCreation)
                    } else {
                        // Existing provider selected
                        self.selected_provider =
                            Some(self.provider_configs[index].clone());
                        self.creation_step = ProfileCreationStep::ConfirmCreate;
                        Ok(ProfileCreatorAction::Refresh)
                    }
                } else {
                    Ok(ProfileCreatorAction::WaitForKeyEvent)
                }
            }
            KeyCode::Esc => {
                self.creation_step = ProfileCreationStep::EnterName;
                Ok(ProfileCreatorAction::Refresh)
            }
            _ => Ok(ProfileCreatorAction::WaitForKeyEvent),
        }
    }

    async fn handle_create_provider(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        if let Some(creator) = &mut self.provider_creator {
            match creator.handle_input(input).await {
                ProviderCreatorAction::Finish(new_config) => {
                    self.provider_configs.push(new_config.clone());
                    self.selected_provider = Some(new_config.clone());
                    self.selected_provider_index =
                        Some(self.provider_configs.len() - 1);
                    self.creation_step = ProfileCreationStep::SelectProvider;
                    self.provider_creator = None;
                    Ok(ProfileCreatorAction::FinishProviderCreation(new_config))
                }
                ProviderCreatorAction::Cancel => {
                    self.creation_step = ProfileCreationStep::SelectProvider;
                    self.provider_creator = None;
                    Ok(ProfileCreatorAction::Refresh)
                }
                ProviderCreatorAction::Refresh => {
                    Ok(ProfileCreatorAction::Refresh)
                }
                ProviderCreatorAction::WaitForKeyEvent => {
                    Ok(ProfileCreatorAction::WaitForKeyEvent)
                }
                ProviderCreatorAction::LoadModels => {
                    creator.load_models().await?;
                    Ok(ProfileCreatorAction::Refresh)
                }
                ProviderCreatorAction::LoadAdditionalSettings => {
                    let model_server =
                        ModelServer::from_str(&creator.provider_type)?;
                    creator.prepare_additional_settings(&model_server);
                    Ok(ProfileCreatorAction::Refresh)
                }
                ProviderCreatorAction::NoAction => {
                    Ok(ProfileCreatorAction::WaitForKeyEvent)
                }
            }
        } else {
            Ok(ProfileCreatorAction::WaitForKeyEvent)
        }
    }

    fn handle_confirm_create(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProfileCreatorAction, ApplicationError> {
        match input.code {
            KeyCode::Enter => {
                self.creation_step = ProfileCreationStep::CreatingProfile;
                Ok(ProfileCreatorAction::CreateProfile)
            }
            KeyCode::Esc => {
                self.creation_step = ProfileCreationStep::SelectProvider;
                Ok(ProfileCreatorAction::Refresh)
            }
            _ => Ok(ProfileCreatorAction::WaitForKeyEvent),
        }
    }

    pub async fn create_profile(
        &mut self,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<UserProfile, ApplicationError> {
        let mut settings = serde_json::Map::new();
        if let Some(selected_config) = &self.selected_provider {
            settings.insert(
                "__TEMPLATE.__MODEL_SERVER".to_string(),
                json!(selected_config.provider_type),
            );
            if let Some(model) = &selected_config.model_identifier {
                settings.insert(
                    "__TEMPLATE.MODEL_IDENTIFIER".to_string(),
                    json!(model),
                );
            }
            for (key, setting) in &selected_config.additional_settings {
                let value = if setting.is_secure {
                    json!({
                        "content": setting.value,
                        "encryption_key": "",
                        "type_info": "string",
                    })
                } else {
                    json!(setting.value)
                };
                settings.insert(format!("__TEMPLATE.{}", key), value);
            }
        }

        let new_profile = db_handler
            .create(&self.new_profile_name, &json!(settings))
            .await?;
        Ok(new_profile)
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        match self.creation_step {
            ProfileCreationStep::EnterName => self.render_enter_name(f, area),
            ProfileCreationStep::SelectProvider => {
                self.render_select_provider(f, area)
            }
            ProfileCreationStep::CreateProvider => {
                if let Some(creator) = &self.provider_creator {
                    creator.render(f, area);
                }
            }
            ProfileCreationStep::ConfirmCreate => {
                self.render_confirm_create(f, area)
            }
            ProfileCreationStep::CreatingProfile => {
                self.render_creating_profile(f, area)
            }
        }
    }

    fn render_enter_name(&self, f: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.new_profile_name.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Enter New Profile Name"),
            );
        f.render_widget(input, area);
    }

    fn render_select_provider(&self, f: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = self
            .provider_configs
            .iter()
            .map(|config| {
                ListItem::new(format!(
                    "{}: {}",
                    config.name, config.provider_type
                ))
            })
            .collect();

        // Add "Create new Provider" option
        items.push(
            ListItem::new("Create new Provider")
                .style(Style::default().fg(Color::Green)),
        );

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Provider"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(self.selected_provider_index);

        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_confirm_create(&self, f: &mut Frame, area: Rect) {
        let mut items =
            vec![ListItem::new(format!("Name: {}", self.new_profile_name))];

        if let Some(config) = &self.selected_provider {
            items.push(ListItem::new(format!(
                "Provider: {}",
                config.provider_type
            )));
            if let Some(model) = &config.model_identifier {
                items.push(ListItem::new(format!("Model: {}", model)));
            }
            for (key, setting) in &config.additional_settings {
                items
                    .push(ListItem::new(format!("{}: {}", key, setting.value)));
            }
        } else {
            items.push(ListItem::new("No provider selected"));
        }

        let confirm_list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm Profile Creation"),
        );
        f.render_widget(confirm_list, area);
    }

    fn render_creating_profile(&self, f: &mut Frame, area: Rect) {
        let content =
            format!("Creating profile '{}'...", self.new_profile_name);
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Creating Profile"),
            );
        f.render_widget(paragraph, area);
    }
}

impl Creator for ProfileCreator {
    fn render(&self, f: &mut Frame, area: Rect) {
        self.render(f, area)
    }
}
