use std::time::Instant;

use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState,
};
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
    scroll_state: ScrollbarState,
    list_state: ListState,
    scroll_position: usize,
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
            scroll_state: ScrollbarState::default(),
            list_state: ListState::default(),
            scroll_position: 0,
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
                Ok(CreatorAction::WaitForKeyEvent)
            }
            _ => Ok(CreatorAction::WaitForKeyEvent),
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

        Ok(CreatorAction::Refresh)
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
                            CreatorAction::Refresh
                        }
                    });
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    result = Some(CreatorAction::Refresh);
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.background_task = None;
                    self.task_start_time = None;
                    result = Some(CreatorAction::Refresh);
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
        state.select(self.selected_provider_index);

        f.render_stateful_widget(list, area, &mut state);
    }

    pub fn render_confirm_create(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        let mut items = Vec::new();

        // Name Section
        items.push(ListItem::new(Line::from(vec![
            Span::styled("Name", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(":"),
        ])));
        items.push(ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &self.new_profile_name,
                Style::default().fg(Color::Cyan),
            ),
        ])));
        items.push(ListItem::new(Line::from("")));

        // Provider Section
        if let Some(config) = &self.selected_provider {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    "Provider",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(":"),
            ])));
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  Type: "),
                Span::styled(
                    &config.provider_type,
                    Style::default().fg(Color::Cyan),
                ),
            ])));

            if let Some(model) = &config.model_identifier {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("  Model: "),
                    Span::styled(model, Style::default().fg(Color::Cyan)),
                ])));
            }

            if !config.additional_settings.is_empty() {
                items.push(ListItem::new(Line::from("  Additional Settings:")));
                for (key, setting) in &config.additional_settings {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(key, Style::default().fg(Color::Yellow)),
                        Span::raw(": "),
                        Span::styled(
                            &setting.value,
                            Style::default().fg(Color::Cyan),
                        ),
                    ])));
                }
            }
        } else {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    "Provider",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(": "),
                Span::styled(
                    "No provider selected",
                    Style::default().fg(Color::Red),
                ),
            ])));
        }

        let list_height = chunks[0].height as usize - 2; // Subtract 2 for borders
        let list = List::new(items.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Profile Details"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(self.scroll_position));

        f.render_stateful_widget(list, chunks[0], &mut list_state);

        // Render scrollbar
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let mut scrollbar_state = ScrollbarState::default()
            .position(self.scroll_position)
            .content_length(items.len())
            .viewport_content_length(list_height);

        f.render_stateful_widget(scrollbar, chunks[0], &mut scrollbar_state);

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

    pub fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }
    }

    pub fn scroll_down(&mut self, max_items: usize) {
        if self.scroll_position < max_items.saturating_sub(1) {
            self.scroll_position += 1;
        }
    }

    fn go_back(&mut self) -> CreatorAction<UserProfile> {
        match self.creation_step {
            ProfileCreationStep::ConfirmCreate => {
                self.creation_step = ProfileCreationStep::SelectProvider;
                CreatorAction::WaitForKeyEvent
            }
            ProfileCreationStep::SelectProvider => {
                self.creation_step = ProfileCreationStep::EnterName;
                CreatorAction::WaitForKeyEvent
            }
            ProfileCreationStep::EnterName => CreatorAction::Cancel,
            _ => CreatorAction::WaitForKeyEvent,
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
