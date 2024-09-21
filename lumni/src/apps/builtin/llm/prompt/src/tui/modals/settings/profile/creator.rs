use std::time::Instant;

use ratatui::layout::Margin;
use serde_json::json;
use tokio::sync::mpsc;

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
    selected_provider: Option<ConfigItem>,
    provider_configs: Vec<ConfigItem>,
    selected_provider_index: usize,
    pub sub_part_creation_state: SubPartCreationState,
    text_area: Option<TextArea<ReadDocument>>,
}

impl ProfileCreator {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let providers = db_handler.list_configuration_items("provider").await?;
        let provider_configs = providers
            .into_iter()
            .map(ConfigItem::DatabaseConfig)
            .collect();

        Ok(Self {
            new_profile_name: String::new(),
            creation_step: ProfileCreationStep::EnterName,
            db_handler,
            background_task: None,
            task_start_time: None,
            selected_provider: None,
            provider_configs,
            selected_provider_index: 0,
            sub_part_creation_state: SubPartCreationState::NotCreating,
            text_area: None,
        })
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
                    self.initialize_confirm_create_state().await;
                }
            }
            KeyCode::Esc | KeyCode::Backspace => {
                return self.go_to_previous_step();
            }
            _ => {}
        };
        Ok(CreatorAction::Continue)
    }

    pub async fn create_profile(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        let (tx, rx) = mpsc::channel(1);
        let mut db_handler = self.db_handler.clone();
        let new_profile_name = self.new_profile_name.clone();
        let selected_provider = self.selected_provider.clone();

        tokio::spawn(async move {
            let mut profile_settings = json!({});

            profile_settings["__name"] = json!(new_profile_name);

            if let Some(ConfigItem::DatabaseConfig(config)) = &selected_provider
            {
                if let Ok(mut provider_settings) = db_handler
                    .get_configuration_parameters(config, MaskMode::Unmask)
                    .await
                {
                    match provider_settings {
                        serde_json::Value::Object(_) => {
                            provider_settings["__name"] = json!(config.name);
                        }
                        _ => {}
                    }
                    profile_settings["__section.provider"] = provider_settings;
                }
            }

            let result = db_handler
                .create_profile(new_profile_name, profile_settings)
                .await;
            let _ = tx.send(BackgroundTaskResult::ProfileCreated(result)).await;
        });

        self.background_task = Some(rx);
        self.task_start_time = Some(Instant::now());
        self.creation_step = ProfileCreationStep::CreatingProfile;

        Ok(CreatorAction::CreateItem)
    }

    pub fn render_select_provider(&self, f: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = self
            .provider_configs
            .iter()
            .map(|config| {
                if let ConfigItem::DatabaseConfig(config) = config {
                    ListItem::new(format!(
                        "{}: {}",
                        config.name, config.section
                    ))
                } else {
                    ListItem::new("Invalid config item")
                }
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

    async fn create_confirm_details(&self) -> Vec<TextLine> {
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
        if let Some(ConfigItem::DatabaseConfig(config)) =
            &self.selected_provider
        {
            let mut provider_line = TextLine::new();
            provider_line.add_segment(
                "Provider:",
                Some(Style::default().add_modifier(Modifier::BOLD)),
            );
            lines.push(provider_line);

            let mut type_line = TextLine::new();
            type_line.add_segment(
                format!("  Type: {}", config.section),
                Some(Style::default().fg(Color::Cyan)),
            );
            lines.push(type_line);

            if let Ok(settings) = self
                .db_handler
                .get_configuration_parameters(config, MaskMode::Mask)
                .await
            {
                if let Some(model) = settings["model_identifier"].as_str() {
                    let mut model_line = TextLine::new();
                    model_line.add_segment(
                        format!("  Model: {}", model),
                        Some(Style::default().fg(Color::Cyan)),
                    );
                    lines.push(model_line);
                }

                if let Some(additional_settings) =
                    settings["additional_settings"].as_object()
                {
                    let mut settings_line = TextLine::new();
                    settings_line.add_segment("  Additional Settings:", None);
                    lines.push(settings_line);

                    for (key, value) in additional_settings {
                        let mut setting_line = TextLine::new();
                        setting_line.add_segment(
                            format!("    • {}: ", key),
                            Some(Style::default().fg(Color::Yellow)),
                        );
                        setting_line.add_segment(
                            value.as_str().unwrap_or("").to_string(),
                            Some(Style::default().fg(Color::Cyan)),
                        );
                        lines.push(setting_line);
                    }
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
}

impl ProfileCreator {
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

    async fn initialize_confirm_create_state(&mut self) {
        let text_lines = self.create_confirm_details().await;
        self.text_area = Some(TextArea::with_read_document(Some(text_lines)));
    }

    pub fn handle_confirm_create(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match input.code {
            KeyCode::Enter => {
                self.text_area = None;
                self.creation_step = ProfileCreationStep::CreatingProfile;
                Ok(CreatorAction::CreateItem)
            }
            KeyCode::Esc => {
                self.text_area = None;
                self.go_to_previous_step()
            }
            _ => {
                // Forward all other key events to the TextAreaState
                if let Some(text_area) = &mut self.text_area {
                    text_area.handle_key_event(input);
                }
                Ok(CreatorAction::Continue)
            }
        }
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

    pub fn render_confirm_create(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        let content_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [Constraint::Min(1), Constraint::Length(1)], // reserve space for scrollbar (to be implemented)
            )
            .split(chunks[0]);

        let text_area_block = Block::default()
            .borders(Borders::ALL)
            .title("Profile Details");

        if let Some(text_area) = &mut self.text_area {
            text_area.render(f, content_area[0].inner(Margin::new(1, 1)));
        } else {
            let fallback_text = Paragraph::new("No profile details available.")
                .style(Style::default().fg(Color::Red));
            f.render_widget(
                fallback_text,
                content_area[0].inner(Margin::new(1, 1)),
            );
        }

        f.render_widget(text_area_block, content_area[0]);

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
