use std::collections::HashMap;
use std::time::Instant;

use ratatui::layout::Margin;
use serde_json::json;
use tokio::sync::mpsc;

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileCreationStep {
    EnterName,
    SelectSection(ProfileSection),
    ConfirmCreate,
    CreatingProfile,
}

pub enum SubPartCreationState {
    NotCreating,
    CreatingSection(ProfileSection, Box<dyn Creator<ConfigItem>>),
}

#[derive(Debug, Clone)]
struct SectionData {
    configs: Vec<ConfigItem>,
    selected_index: usize,
    selected_item: Option<ConfigItem>,
}
pub struct ProfileCreator {
    new_profile_name: String,
    creation_step: ProfileCreationStep,
    db_handler: UserProfileDbHandler,
    background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    task_start_time: Option<Instant>,
    sections: HashMap<ProfileSection, SectionData>,
    section_order: Vec<ProfileSection>,
    current_section_index: usize,
    sub_part_creation_state: SubPartCreationState,
    text_area: Option<TextArea<ReadDocument>>,
    editing_profile: Option<UserProfile>,
}

impl ProfileCreator {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        let mut sections = HashMap::new();
        let section_order =
            vec![ProfileSection::Provider, ProfileSection::Prompt];

        for section in &section_order {
            let configs = db_handler
                .list_configuration_items(section.as_str())
                .await?
                .into_iter()
                .map(ConfigItem::DatabaseConfig)
                .collect();
            sections.insert(
                section.clone(),
                SectionData {
                    configs,
                    selected_index: 0,
                    selected_item: None,
                },
            );
        }

        Ok(Self {
            new_profile_name: String::new(),
            creation_step: ProfileCreationStep::EnterName,
            db_handler,
            background_task: None,
            task_start_time: None,
            sections,
            section_order,
            current_section_index: 0,
            sub_part_creation_state: SubPartCreationState::NotCreating,
            text_area: None,
            editing_profile: None,
        })
    }

    pub fn set_editing_mode(
        &mut self,
        profile: UserProfile,
        step: ProfileCreationStep,
    ) {
        self.editing_profile = Some(profile.clone());
        self.new_profile_name = profile.name.clone();
        self.creation_step = step;
    }

    pub fn render_creator(&mut self, f: &mut Frame, area: Rect) {
        match &mut self.sub_part_creation_state {
            SubPartCreationState::NotCreating => match &self.creation_step {
                ProfileCreationStep::EnterName => {
                    self.render_enter_name(f, area)
                }
                ProfileCreationStep::SelectSection(section) => {
                    self.render_select_section(f, area, section)
                }
                ProfileCreationStep::ConfirmCreate => {
                    self.render_confirm_create(f, area)
                }
                ProfileCreationStep::CreatingProfile => {
                    self.render_creating_profile(f, area)
                }
            },
            SubPartCreationState::CreatingSection(_, creator) => {
                creator.render(f, area)
            }
        }
    }

    pub async fn create_profile(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        let (tx, rx) = mpsc::channel(1);
        let mut db_handler = self.db_handler.clone();
        let new_profile_name = self.new_profile_name.clone();
        let sections = self.sections.clone();

        tokio::spawn(async move {
            let mut profile_settings = json!({});

            for (section, section_data) in sections {
                if let Some(ConfigItem::DatabaseConfig(config)) =
                    &section_data.selected_item
                {
                    if let Ok(section_configuration) = db_handler
                        .get_configuration_parameters(&config, MaskMode::Unmask)
                        .await
                    {
                        let section_key =
                            format!("__section.{}", section.as_str());
                        profile_settings[section_key] = section_configuration;
                    }
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

    pub async fn update_profile(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        if let Some(profile) = &self.editing_profile {
            let mut profile_settings = json!({
                "name": self.new_profile_name,
            });

            for (section_name, section_data) in &self.sections {
                if let Some(ConfigItem::DatabaseConfig(config)) =
                    &section_data.selected_item
                {
                    if let Ok(section_configuration) = self
                        .db_handler
                        .get_configuration_parameters(config, MaskMode::Unmask)
                        .await
                    {
                        let section_key = format!("__section.{}", section_name);
                        profile_settings[section_key] = section_configuration;
                    }
                }
            }

            self.db_handler
                .update_configuration_item(&profile.into(), &profile_settings)
                .await?;

            // Fetch the updated profile
            let updated_profile =
                self.db_handler.get_profile_by_id(profile.id).await?.ok_or(
                    ApplicationError::InvalidState(
                        "Updated profile not found".to_string(),
                    ),
                )?;

            Ok(CreatorAction::Finish(updated_profile))
        } else {
            Err(ApplicationError::InvalidState(
                "No profile selected for update".to_string(),
            ))
        }
    }

    async fn create_confirm_details(&self) -> Vec<TextLine> {
        let mut lines = Vec::new();

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

        // Iterate through sections
        for section in &self.section_order {
            if let Some(section_data) = self.sections.get(section) {
                if let Some(ConfigItem::DatabaseConfig(config)) =
                    &section_data.selected_item
                {
                    let mut section_line = TextLine::new();
                    section_line.add_segment(
                        format!("{}:", section),
                        Some(Style::default().add_modifier(Modifier::BOLD)),
                    );
                    lines.push(section_line);

                    let mut name_line = TextLine::new();
                    name_line.add_segment(
                        format!("  Name: {}", config.name),
                        Some(Style::default().fg(Color::Cyan)),
                    );
                    lines.push(name_line);

                    if let Ok(settings) = self
                        .db_handler
                        .get_configuration_parameters(config, MaskMode::Mask)
                        .await
                    {
                        match section {
                            ProfileSection::Provider => {
                                if let Some(model) =
                                    settings["model_identifier"].as_str()
                                {
                                    let mut model_line = TextLine::new();
                                    model_line.add_segment(
                                        format!("  Model: {}", model),
                                        Some(Style::default().fg(Color::Cyan)),
                                    );
                                    lines.push(model_line);
                                }
                            }
                            ProfileSection::Prompt => {
                                if let Some(content) =
                                    settings["content"].as_str()
                                {
                                    let mut content_line = TextLine::new();
                                    content_line.add_segment(
                                        format!("  Content: {}", content),
                                        Some(Style::default().fg(Color::Cyan)),
                                    );
                                    lines.push(content_line);
                                }
                            }
                        }
                    }
                } else {
                    let mut no_selection_line = TextLine::new();
                    no_selection_line.add_segment(
                        format!("{}: ", section),
                        Some(Style::default().add_modifier(Modifier::BOLD)),
                    );
                    no_selection_line.add_segment(
                        format!("No {} selected", section),
                        Some(Style::default().fg(Color::Red)),
                    );
                    lines.push(no_selection_line);
                }
                lines.push(TextLine::new()); // Empty line for spacing
            }
        }

        lines
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
                        if let ProfileCreationStep::EnterName =
                            self.creation_step
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

                match &self.creation_step {
                    ProfileCreationStep::EnterName => {
                        self.handle_enter_name(input).await
                    }
                    ProfileCreationStep::SelectSection(section) => {
                        let section_clone = section.clone();
                        self.handle_select_section(input, &section_clone).await
                    }
                    ProfileCreationStep::ConfirmCreate => {
                        self.handle_confirm_create(input).await
                    }
                    ProfileCreationStep::CreatingProfile => {
                        Ok(CreatorAction::Continue)
                    }
                }
            }
            SubPartCreationState::CreatingSection(section, creator) => {
                if input.code == KeyCode::Esc {
                    let section_clone = section.clone();
                    self.sub_part_creation_state =
                        SubPartCreationState::NotCreating;
                    self.creation_step =
                        ProfileCreationStep::SelectSection(section_clone);
                    return Ok(CreatorAction::Continue);
                }

                let result = creator.handle_input(input).await?;
                match result {
                    CreatorAction::Finish(new_config) => {
                        let section_clone = section.clone();
                        if let Some(section_data) =
                            self.sections.get_mut(&section_clone)
                        {
                            section_data.configs.push(new_config.clone());
                            section_data.selected_item = Some(new_config);
                            section_data.selected_index =
                                section_data.configs.len() - 1;
                        }
                        self.sub_part_creation_state =
                            SubPartCreationState::NotCreating;
                        self.creation_step =
                            ProfileCreationStep::SelectSection(section_clone);
                        Ok(CreatorAction::Continue)
                    }
                    CreatorAction::Cancel => {
                        let section_clone = section.clone();
                        self.sub_part_creation_state =
                            SubPartCreationState::NotCreating;
                        self.creation_step =
                            ProfileCreationStep::SelectSection(section_clone);
                        Ok(CreatorAction::Continue)
                    }
                    _ => Ok(CreatorAction::Continue),
                }
            }
        }
    }

    async fn handle_select_section(
        &mut self,
        input: KeyEvent,
        section: &ProfileSection,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        if let Some(section_data) = self.sections.get_mut(section) {
            match input.code {
                KeyCode::Up => {
                    if section_data.selected_index > 0 {
                        section_data.selected_index -= 1;
                    } else {
                        section_data.selected_index =
                            section_data.configs.len(); // Wrap to "Create new" option
                    }
                }
                KeyCode::Down => {
                    if section_data.selected_index < section_data.configs.len()
                    {
                        section_data.selected_index += 1;
                    } else {
                        section_data.selected_index = 0; // Wrap to first item
                    }
                }
                KeyCode::Enter => {
                    if section_data.selected_index == section_data.configs.len()
                    {
                        // "Create new" option selected
                        let creator =
                            self.create_section_creator(section).await?;
                        self.sub_part_creation_state =
                            SubPartCreationState::CreatingSection(
                                section.clone(),
                                creator,
                            );
                    } else {
                        // Existing item selected
                        section_data.selected_item = Some(
                            section_data.configs[section_data.selected_index]
                                .clone(),
                        );
                        return self.go_to_next_step().await;
                    }
                }
                KeyCode::Esc | KeyCode::Backspace => {
                    return self.go_to_previous_step();
                }
                _ => {}
            };
        }
        Ok(CreatorAction::Continue)
    }

    fn render_select_section(
        &self,
        f: &mut Frame,
        area: Rect,
        section: &ProfileSection,
    ) {
        if let Some(section_data) = self.sections.get(section) {
            let mut items: Vec<ListItem> = section_data
                .configs
                .iter()
                .map(|config| {
                    if let ConfigItem::DatabaseConfig(config) = config {
                        ListItem::new(config.name.clone())
                    } else {
                        ListItem::new("Invalid config item")
                    }
                })
                .collect();

            items.push(
                ListItem::new(format!("Create new {}", section))
                    .style(Style::default().fg(Color::Green)),
            );

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Select {}", section)),
                )
                .highlight_style(
                    Style::default().add_modifier(Modifier::REVERSED),
                )
                .highlight_symbol("> ");

            let mut state = ListState::default();
            state.select(Some(section_data.selected_index));

            f.render_stateful_widget(list, area, &mut state);
        }
    }

    async fn create_section_creator(
        &self,
        section: &ProfileSection,
    ) -> Result<Box<dyn Creator<ConfigItem>>, ApplicationError> {
        match section {
            ProfileSection::Provider => {
                let provider_creator =
                    ProviderCreator::new(self.db_handler.clone()).await?;
                Ok(Box::new(provider_creator) as Box<dyn Creator<ConfigItem>>)
            }
            ProfileSection::Prompt => {
                let prompt_creator =
                    PromptCreator::new(self.db_handler.clone());
                Ok(Box::new(prompt_creator) as Box<dyn Creator<ConfigItem>>)
            }
        }
    }

    async fn handle_enter_name(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match input.code {
            KeyCode::Char(c) => {
                self.new_profile_name.push(c);
                Ok(CreatorAction::Continue)
            }
            KeyCode::Enter => self.go_to_next_step().await,
            _ => Ok(CreatorAction::Continue),
        }
    }

    async fn initialize_confirm_create_state(&mut self) {
        let text_lines = self.create_confirm_details().await;
        self.text_area = Some(TextArea::with_read_document(Some(text_lines)));
    }

    async fn handle_confirm_create(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        match input.code {
            KeyCode::Enter => {
                self.text_area = None;
                if self.editing_profile.is_some() {
                    self.update_profile().await
                } else {
                    self.go_to_next_step().await
                }
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

    fn render_confirm_create(&mut self, f: &mut Frame, area: Rect) {
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

        let action_button = if self.editing_profile.is_some() {
            Paragraph::new("[ Update Profile ]")
                .style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center)
        } else {
            Paragraph::new("[ Create Profile ]")
                .style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center)
        };
        f.render_widget(action_button, button_chunks[1]);
    }

    fn go_to_previous_step(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        if self.editing_profile.is_some() {
            self.editing_profile = None;
            Ok(CreatorAction::Cancel)
        } else {
            match &self.creation_step {
                ProfileCreationStep::EnterName => Ok(CreatorAction::Cancel),
                ProfileCreationStep::SelectSection(_) => {
                    if self.current_section_index == 0 {
                        self.creation_step = ProfileCreationStep::EnterName;
                    } else {
                        self.current_section_index -= 1;
                        self.creation_step = ProfileCreationStep::SelectSection(
                            self.section_order[self.current_section_index]
                                .clone(),
                        );
                    }
                    Ok(CreatorAction::Continue)
                }
                ProfileCreationStep::ConfirmCreate => {
                    self.current_section_index = self.section_order.len() - 1;
                    self.creation_step = ProfileCreationStep::SelectSection(
                        self.section_order[self.current_section_index].clone(),
                    );
                    Ok(CreatorAction::Continue)
                }
                ProfileCreationStep::CreatingProfile => {
                    self.creation_step = ProfileCreationStep::ConfirmCreate;
                    Ok(CreatorAction::Continue)
                }
            }
        }
    }

    async fn go_to_next_step(
        &mut self,
    ) -> Result<CreatorAction<UserProfile>, ApplicationError> {
        if self.editing_profile.is_some() {
            self.creation_step = ProfileCreationStep::ConfirmCreate;
            self.initialize_confirm_create_state().await;
            Ok(CreatorAction::Continue)
        } else {
            match &self.creation_step {
                ProfileCreationStep::EnterName => {
                    if !self.new_profile_name.is_empty() {
                        self.current_section_index = 0;
                        self.creation_step = ProfileCreationStep::SelectSection(
                            self.section_order[0].clone(),
                        );
                    }
                    Ok(CreatorAction::Continue)
                }
                ProfileCreationStep::SelectSection(_) => {
                    self.current_section_index += 1;
                    if self.current_section_index < self.section_order.len() {
                        self.creation_step = ProfileCreationStep::SelectSection(
                            self.section_order[self.current_section_index]
                                .clone(),
                        );
                        Ok(CreatorAction::Continue)
                    } else {
                        self.creation_step = ProfileCreationStep::ConfirmCreate;
                        self.initialize_confirm_create_state().await;
                        Ok(CreatorAction::Continue)
                    }
                }
                ProfileCreationStep::ConfirmCreate => {
                    self.creation_step = ProfileCreationStep::CreatingProfile;
                    Ok(CreatorAction::CreateItem)
                }
                ProfileCreationStep::CreatingProfile => {
                    unreachable!(
                        "Unexpected state: no next step after CreatingProfile"
                    );
                }
            }
        }
    }

    fn render_creating_profile(&self, f: &mut Frame, area: Rect) {
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
