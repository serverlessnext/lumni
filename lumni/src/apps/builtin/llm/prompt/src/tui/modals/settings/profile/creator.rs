use std::time::Instant;

use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use serde_json::json;
use tokio::sync::mpsc;

use super::provider::ProviderCreator;
use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileCreationStep {
    EnterName,
    SelectProvider,
    CreateProvider,
    ConfirmCreate,
    CreatingProfile,
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
    pub provider_creator: Option<ProviderCreator>,
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

    pub fn handle_enter_name(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match input.code {
            KeyCode::Char(c) => {
                self.new_profile_name.push(c);
                Ok(CreatorAction::Refresh)
            }
            KeyCode::Backspace => {
                self.new_profile_name.pop();
                Ok(CreatorAction::Refresh)
            }
            KeyCode::Enter => {
                if !self.new_profile_name.is_empty() {
                    self.creation_step = ProfileCreationStep::SelectProvider;
                    Ok(CreatorAction::Refresh)
                } else {
                    Ok(CreatorAction::WaitForKeyEvent)
                }
            }
            KeyCode::Esc => Ok(CreatorAction::Cancel),
            _ => Ok(CreatorAction::WaitForKeyEvent),
        }
    }

    pub async fn handle_select_provider(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
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
                Ok(CreatorAction::Refresh)
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
                Ok(CreatorAction::Refresh)
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
                        Ok(CreatorAction::SwitchToProviderCreation)
                    } else {
                        // Existing provider selected
                        self.selected_provider =
                            Some(self.provider_configs[index].clone());
                        self.creation_step = ProfileCreationStep::ConfirmCreate;
                        Ok(CreatorAction::Refresh)
                    }
                } else {
                    Ok(CreatorAction::WaitForKeyEvent)
                }
            }
            KeyCode::Esc => {
                self.creation_step = ProfileCreationStep::EnterName;
                Ok(CreatorAction::Refresh)
            }
            _ => Ok(CreatorAction::WaitForKeyEvent),
        }
    }

    pub async fn handle_create_provider(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        if let Some(creator) = &mut self.provider_creator {
            match creator.handle_input(input).await? {
                CreatorAction::Finish(new_config) => {
                    self.provider_configs.push(new_config.clone());
                    self.selected_provider = Some(new_config);
                    self.selected_provider_index =
                        Some(self.provider_configs.len() - 1);
                    self.creation_step = ProfileCreationStep::ConfirmCreate;
                    self.provider_creator = None;
                    Ok(CreatorAction::Refresh)
                }
                CreatorAction::Cancel => {
                    self.creation_step = ProfileCreationStep::SelectProvider;
                    self.provider_creator = None;
                    Ok(CreatorAction::Refresh)
                }
                CreatorAction::Refresh => Ok(CreatorAction::Refresh),
                CreatorAction::WaitForKeyEvent => {
                    Ok(CreatorAction::WaitForKeyEvent)
                }
                CreatorAction::LoadAdditionalSettings => {
                    let model_server =
                        ModelServer::from_str(&creator.provider_type)?;
                    creator.prepare_additional_settings(&model_server);
                    Ok(CreatorAction::Refresh)
                }
                CreatorAction::CreateItem => {
                    // This is the case we need to handle for the confirmation step
                    match creator.create_item().await? {
                        CreatorAction::Finish(new_config) => {
                            self.provider_configs.push(new_config.clone());
                            self.selected_provider = Some(new_config);
                            self.selected_provider_index =
                                Some(self.provider_configs.len() - 1);
                            self.creation_step =
                                ProfileCreationStep::ConfirmCreate;
                            self.provider_creator = None;
                            Ok(CreatorAction::Refresh)
                        }
                        _ => Ok(CreatorAction::WaitForKeyEvent),
                    }
                }
                _ => Ok(CreatorAction::WaitForKeyEvent),
            }
        } else {
            Ok(CreatorAction::WaitForKeyEvent)
        }
    }

    pub fn handle_confirm_create(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match input.code {
            KeyCode::Enter => {
                self.creation_step = ProfileCreationStep::CreatingProfile;
                Ok(CreatorAction::CreateItem)
            }
            KeyCode::Esc => {
                self.creation_step = ProfileCreationStep::SelectProvider;
                Ok(CreatorAction::Refresh)
            }
            _ => Ok(CreatorAction::WaitForKeyEvent),
        }
    }

    pub async fn create_profile(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        let new_profile = self.create_profile_internal().await?;
        Ok(CreatorAction::Finish(new_profile))
    }

    async fn create_profile_internal(
        &mut self,
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

        let new_profile = self
            .db_handler
            .create(&self.new_profile_name, &json!(settings))
            .await?;
        Ok(new_profile)
    }

    pub fn render_enter_name(&self, f: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.new_profile_name.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Enter New Profile Name"),
            );
        f.render_widget(input, area);
    }

    pub fn render_select_provider(&self, f: &mut Frame, area: Rect) {
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

    pub fn render_confirm_create(&self, f: &mut Frame, area: Rect) {
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

    pub fn render_creating_profile(&self, f: &mut Frame, area: Rect) {
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
