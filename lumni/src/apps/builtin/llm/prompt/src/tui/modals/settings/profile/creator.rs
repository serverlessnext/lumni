use std::time::Instant;

use ratatui::layout::{Alignment, Constraint, Direction, Layout};
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
    ConfirmCreate,
    CreatingProfile,
}

#[derive(Debug, Clone)]
pub enum SubPartCreationState {
    NotCreating,
    CreatingProvider(ProviderCreator),
}

pub struct ProfileCreator {
    new_profile_name: String,
    pub creation_step: ProfileCreationStep,
    db_handler: UserProfileDbHandler,
    pub background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    task_start_time: Option<Instant>,
    selected_provider: Option<ProviderConfig>,
    provider_configs: Vec<ProviderConfig>,
    selected_provider_index: usize,
    pub sub_part_creation_state: SubPartCreationState,
}

impl ProfileCreator {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        // Load existing provider configs
        let provider_configs = db_handler.load_provider_configs().await?;

        Ok(Self {
            new_profile_name: String::new(),
            creation_step: ProfileCreationStep::EnterName,
            db_handler: db_handler.clone(),
            background_task: None,
            task_start_time: None,
            selected_provider: None,
            provider_configs,
            selected_provider_index: 0,
            sub_part_creation_state: SubPartCreationState::NotCreating,
        })
    }

    pub async fn handle_key_event(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match &mut self.sub_part_creation_state {
            SubPartCreationState::NotCreating => {
                match input.code {
                    KeyCode::Esc => return self.go_to_previous_step(),
                    KeyCode::Backspace => {
                        if self.creation_step == ProfileCreationStep::EnterName
                        {
                            if self.new_profile_name.is_empty() {
                                return self.go_to_previous_step();
                            } else {
                                self.new_profile_name.pop();
                                return Ok(CreatorAction::Continue);
                            }
                        } else {
                            return self.go_to_previous_step();
                        }
                    }
                    _ => {}
                }

                match self.creation_step {
                    ProfileCreationStep::EnterName => {
                        self.handle_enter_name(input)
                    }
                    ProfileCreationStep::SelectProvider => {
                        self.handle_select_provider(input).await
                    }
                    ProfileCreationStep::ConfirmCreate => {
                        self.handle_confirm_create(input)
                    }
                    ProfileCreationStep::CreatingProfile => {
                        Ok(CreatorAction::Continue)
                    }
                }
            }
            SubPartCreationState::CreatingProvider(creator) => {
                match input.code {
                    KeyCode::Esc => {
                        self.sub_part_creation_state =
                            SubPartCreationState::NotCreating;
                        self.creation_step =
                            ProfileCreationStep::SelectProvider;
                        return Ok(CreatorAction::Continue);
                    }
                    _ => {}
                }

                let result = creator.handle_input(input).await?;
                match result {
                    CreatorAction::Finish(new_config) => {
                        self.provider_configs.push(new_config.clone());
                        self.selected_provider = Some(new_config);
                        self.selected_provider_index =
                            self.provider_configs.len() - 1;
                        self.sub_part_creation_state =
                            SubPartCreationState::NotCreating;
                        self.creation_step =
                            ProfileCreationStep::SelectProvider;
                        Ok(CreatorAction::Continue)
                    }
                    CreatorAction::Cancel => {
                        self.sub_part_creation_state =
                            SubPartCreationState::NotCreating;
                        self.creation_step =
                            ProfileCreationStep::SelectProvider;
                        Ok(CreatorAction::Continue)
                    }
                    _ => Ok(CreatorAction::Continue),
                }
            }
        }
    }

    fn handle_enter_name(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match input.code {
            KeyCode::Char(c) => {
                self.new_profile_name.push(c);
                Ok(CreatorAction::Continue)
            }
            KeyCode::Enter => {
                if !self.new_profile_name.is_empty() {
                    self.creation_step = ProfileCreationStep::SelectProvider;
                }
                Ok(CreatorAction::Continue)
            }
            _ => Ok(CreatorAction::Continue),
        }
    }

    pub async fn handle_select_provider(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match input.code {
            KeyCode::Up => {
                if self.selected_provider_index > 0 {
                    self.selected_provider_index -= 1;
                } else {
                    self.selected_provider_index = self.provider_configs.len(); // Wrap to "Create new Provider" option
                }
            }
            KeyCode::Down => {
                if self.selected_provider_index < self.provider_configs.len() {
                    self.selected_provider_index += 1;
                } else {
                    self.selected_provider_index = 0; // Wrap to first provider
                }
            }
            KeyCode::Enter => {
                if self.selected_provider_index == self.provider_configs.len() {
                    // "Create new Provider" option selected
                    let creator =
                        ProviderCreator::new(self.db_handler.clone()).await?;
                    self.sub_part_creation_state =
                        SubPartCreationState::CreatingProvider(creator);
                } else {
                    // Existing provider selected
                    self.selected_provider = Some(
                        self.provider_configs[self.selected_provider_index]
                            .clone(),
                    );
                    self.creation_step = ProfileCreationStep::ConfirmCreate;
                }
            }
            KeyCode::Esc | KeyCode::Backspace => {
                return self.go_to_previous_step();
            }
            _ => {}
        };
        Ok(CreatorAction::Continue)
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
            _ => Ok(CreatorAction::Continue),
        }
    }

    pub async fn create_profile(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        let (tx, rx) = mpsc::channel(1);
        let mut db_handler = self.db_handler.clone();
        let new_profile_name = self.new_profile_name.clone();
        let selected_provider = self.selected_provider.clone();

        tokio::spawn(async move {
            let mut settings = serde_json::Map::new();
            if let Some(selected_config) = &selected_provider {
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

            let result =
                db_handler.create(&new_profile_name, &json!(settings)).await;
            let _ = tx.send(BackgroundTaskResult::ProfileCreated(result)).await;
        });

        self.background_task = Some(rx);
        self.task_start_time = Some(Instant::now());
        self.creation_step = ProfileCreationStep::CreatingProfile;

        Ok(CreatorAction::CreateItem)
    }

    pub fn check_profile_creation_status(
        &mut self,
    ) -> Option<CreatorAction<UserProfile>> {
        let mut result = None;

        if let Some(rx) = &mut self.background_task {
            match rx.try_recv() {
                Ok(BackgroundTaskResult::ProfileCreated(profile_result)) => {
                    self.background_task = None;
                    self.task_start_time = None;
                    result = Some(match profile_result {
                        Ok(new_profile) => CreatorAction::Finish(new_profile),
                        Err(e) => {
                            log::error!("Failed to create profile: {}", e);
                            CreatorAction::CreateItem
                        }
                    });
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    result = Some(CreatorAction::CreateItem);
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.background_task = None;
                    self.task_start_time = None;
                    result = Some(CreatorAction::CreateItem);
                }
            }
        }
        result
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
        state.select(Some(self.selected_provider_index));

        f.render_stateful_widget(list, area, &mut state);
    }

    pub fn render_confirm_create(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        let content_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [Constraint::Min(1), Constraint::Length(1)], // reserve space for scrollbar ( to be implemented )
            )
            .split(chunks[0]);

        let text_lines = self.create_confirm_details();
        let mut text_area = ResponseWindow::new(Some(text_lines));

        let text_area_block = Block::default()
            .borders(Borders::ALL)
            .title("Profile Details");
        let text_area_widget =
            text_area.widget(&content_area[0]).block(text_area_block);
        f.render_widget(text_area_widget, content_area[0]);

        // Render buttons
        let button_constraints =
            [Constraint::Percentage(50), Constraint::Percentage(50)];
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(button_constraints)
            .split(chunks[1]);

        let back_button = Paragraph::new("[ Back ]")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(back_button, button_chunks[0]);

        let create_button = Paragraph::new("[ Create Profile ]")
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(create_button, button_chunks[1]);
    }

    fn create_confirm_details(&self) -> Vec<TextLine> {
        let mut lines = Vec::new();

        // Name Section
        let mut name_line = TextLine::new();
        name_line.add_segment(
            "Name:",
            Some(Style::default().add_modifier(Modifier::BOLD)),
        );
        lines.push(name_line);

        let mut name_value_line = TextLine::new();
        name_value_line.add_segment(
            format!("  {}", self.new_profile_name),
            Some(Style::default().fg(Color::Cyan)),
        );
        lines.push(name_value_line);

        lines.push(TextLine::new()); // Empty line for spacing

        // Provider Section
        if let Some(config) = &self.selected_provider {
            let mut provider_line = TextLine::new();
            provider_line.add_segment(
                "Provider:",
                Some(Style::default().add_modifier(Modifier::BOLD)),
            );
            lines.push(provider_line);

            let mut type_line = TextLine::new();
            type_line.add_segment(
                format!("  Type: {}", config.provider_type),
                Some(Style::default().fg(Color::Cyan)),
            );
            lines.push(type_line);

            if let Some(model) = &config.model_identifier {
                let mut model_line = TextLine::new();
                model_line.add_segment(
                    format!("  Model: {}", model),
                    Some(Style::default().fg(Color::Cyan)),
                );
                lines.push(model_line);
            }

            if !config.additional_settings.is_empty() {
                let mut settings_line = TextLine::new();
                settings_line.add_segment("  Additional Settings:", None);
                lines.push(settings_line);

                for (key, setting) in &config.additional_settings {
                    let mut setting_line = TextLine::new();
                    setting_line.add_segment(
                        format!("    • {}: ", key),
                        Some(Style::default().fg(Color::Yellow)),
                    );
                    setting_line.add_segment(
                        &setting.value,
                        Some(Style::default().fg(Color::Cyan)),
                    );
                    lines.push(setting_line);
                }
            }
        } else {
            let mut no_provider_line = TextLine::new();
            no_provider_line.add_segment(
                "Provider: ",
                Some(Style::default().add_modifier(Modifier::BOLD)),
            );
            no_provider_line.add_segment(
                "No provider selected",
                Some(Style::default().fg(Color::Red)),
            );
            lines.push(no_provider_line);
        }

        lines
    }

    pub fn go_to_previous_step(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match self.creation_step {
            ProfileCreationStep::EnterName => Ok(CreatorAction::Cancel),
            ProfileCreationStep::SelectProvider => {
                self.creation_step = ProfileCreationStep::EnterName;
                Ok(CreatorAction::Continue)
            }
            ProfileCreationStep::ConfirmCreate => {
                self.creation_step = ProfileCreationStep::SelectProvider;
                Ok(CreatorAction::Continue)
            }
            ProfileCreationStep::CreatingProfile => {
                self.creation_step = ProfileCreationStep::ConfirmCreate;
                Ok(CreatorAction::Continue)
            }
        }
    }

    pub fn render_creating_profile(&self, f: &mut Frame, area: Rect) {
        let elapsed = self
            .task_start_time
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);

        let content = format!(
            "Creating profile '{}' ... ({} seconds)",
            self.new_profile_name, elapsed
        );

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
