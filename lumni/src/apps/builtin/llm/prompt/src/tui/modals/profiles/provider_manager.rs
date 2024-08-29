use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: Option<usize>,
    pub name: String,
    pub provider_type: String,
    pub model_identifier: Option<String>,
    pub additional_settings: HashMap<String, AdditionalSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditionalSetting {
    pub name: String,
    pub display_name: String,
    pub value: String,
    pub is_secure: bool,
    pub placeholder: String,
}

pub struct ProviderManager {
    configs: Vec<ProviderConfig>,
    selected_index: Option<usize>,
    db_handler: UserProfileDbHandler,
    provider_creator: Option<ProviderCreator>,
}

impl ProviderManager {
    pub fn new(db_handler: UserProfileDbHandler) -> Self {
        Self {
            configs: Vec::new(),
            selected_index: None,
            db_handler,
            provider_creator: None,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if let Some(creator) = &self.provider_creator {
            creator.render(f, area);
        } else {
            self.render_provider_list(f, area);
        }
    }

    fn render_provider_list(&self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        if self.configs.is_empty() {
            items.push(ListItem::new("No providers configured"));
        } else {
            for (index, config) in self.configs.iter().enumerate() {
                let content =
                    format!("{}: {}", config.name, config.provider_type);
                let style = if Some(index) == self.selected_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                items.push(ListItem::new(content).style(style));
            }
        }

        items.push(
            ListItem::new("Create New Provider")
                .style(Style::default().fg(Color::Green)),
        );

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select or Create Provider"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(self.selected_index);

        f.render_stateful_widget(list, area, &mut state);
    }

    pub async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> Result<ProviderManagerAction, ApplicationError> {
        if let Some(creator) = &mut self.provider_creator {
            match creator.handle_input(input).await {
                ProviderCreatorAction::Finish(new_config) => {
                    self.configs.push(new_config);
                    self.provider_creator = None;
                    Ok(ProviderManagerAction::Refresh)
                }
                ProviderCreatorAction::Cancel => {
                    self.provider_creator = None;
                    Ok(ProviderManagerAction::Refresh)
                }
                ProviderCreatorAction::LoadModels => {
                    creator.load_models().await?;
                    Ok(ProviderManagerAction::Refresh)
                }
                ProviderCreatorAction::LoadAdditionalSettings => {
                    let model_server =
                        ModelServer::from_str(&creator.provider_type)?;
                    creator.prepare_additional_settings(&model_server);
                    Ok(ProviderManagerAction::Refresh)
                }
                ProviderCreatorAction::Refresh => {
                    Ok(ProviderManagerAction::Refresh)
                }
                ProviderCreatorAction::NoAction => {
                    Ok(ProviderManagerAction::NoAction)
                }
            }
        } else {
            match input.code {
                KeyCode::Up => Ok(self.move_selection_up()),
                KeyCode::Down => Ok(self.move_selection_down()),
                KeyCode::Enter => Ok(self.select_or_create_provider()),
                _ => Ok(ProviderManagerAction::NoAction),
            }
        }
    }

    fn move_selection_up(&mut self) -> ProviderManagerAction {
        if self.configs.is_empty() {
            return ProviderManagerAction::NoAction;
        }
        if let Some(index) = self.selected_index.as_mut() {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = self.configs.len(); // Wrap to "Create New Provider"
            }
        } else {
            self.selected_index = Some(self.configs.len()); // Select "Create New Provider"
        }
        ProviderManagerAction::Refresh
    }

    fn move_selection_down(&mut self) -> ProviderManagerAction {
        if self.configs.is_empty() {
            return ProviderManagerAction::NoAction;
        }
        if let Some(index) = self.selected_index.as_mut() {
            if *index < self.configs.len() {
                *index += 1;
            } else {
                *index = 0; // Wrap to first provider
            }
        } else {
            self.selected_index = Some(0);
        }
        ProviderManagerAction::Refresh
    }

    fn select_or_create_provider(&mut self) -> ProviderManagerAction {
        if self.configs.is_empty()
            || self.selected_index == Some(self.configs.len())
        {
            self.provider_creator =
                Some(ProviderCreator::new(self.db_handler.clone()));
            ProviderManagerAction::Refresh
        } else if self.selected_index.is_some() {
            ProviderManagerAction::ProviderSelected
        } else {
            ProviderManagerAction::NoAction
        }
    }

    pub fn get_selected_provider(&self) -> Option<&ProviderConfig> {
        self.selected_index
            .and_then(|index| self.configs.get(index))
    }

    pub async fn load_configs(&mut self) -> Result<(), ApplicationError> {
        self.configs = self.db_handler.load_provider_configs().await?;
        if !self.configs.is_empty() {
            self.selected_index = Some(0);
        } else {
            self.selected_index = None;
        }
        Ok(())
    }
}

pub struct ProviderCreator {
    name: String,
    provider_type: String,
    model_identifier: Option<String>,
    additional_settings: HashMap<String, AdditionalSetting>,
    db_handler: UserProfileDbHandler,
    current_step: ProviderCreationStep,
    available_models: Vec<ModelSpec>,
    selected_model_index: Option<usize>,
    current_setting_key: Option<String>,
    edit_buffer: String,
    is_editing: bool,
    model_fetch_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum ProviderCreationStep {
    EnterName,
    SelectProviderType,
    SelectModel,
    ConfigureSettings,
    Confirm,
}

impl ProviderCreator {
    pub fn new(db_handler: UserProfileDbHandler) -> Self {
        Self {
            name: String::new(),
            provider_type: String::new(),
            model_identifier: None,
            additional_settings: HashMap::new(),
            db_handler,
            current_step: ProviderCreationStep::EnterName,
            available_models: Vec::new(),
            selected_model_index: None,
            current_setting_key: None,
            edit_buffer: String::new(),
            is_editing: false,
            model_fetch_error: None,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        match self.current_step {
            ProviderCreationStep::EnterName => self.render_enter_name(f, area),
            ProviderCreationStep::SelectProviderType => {
                self.render_select_provider_type(f, area)
            }
            ProviderCreationStep::SelectModel => {
                self.render_select_model(f, area)
            }
            ProviderCreationStep::ConfigureSettings => {
                self.render_configure_settings(f, area)
            }
            ProviderCreationStep::Confirm => self.render_confirm(f, area),
        }
    }

    pub async fn handle_input(
        &mut self,
        input: KeyEvent,
    ) -> ProviderCreatorAction {
        match self.current_step {
            ProviderCreationStep::EnterName => self.handle_enter_name(input),
            ProviderCreationStep::SelectProviderType => {
                self.handle_select_provider_type(input).await
            }
            ProviderCreationStep::SelectModel => {
                self.handle_select_model(input).await
            }
            ProviderCreationStep::ConfigureSettings => {
                self.handle_configure_settings(input)
            }
            ProviderCreationStep::Confirm => self.handle_confirm(input).await,
        }
    }

    fn render_enter_name(&self, f: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.name.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Enter Provider Name"),
            );
        f.render_widget(input, area);
    }

    fn render_select_provider_type(&self, f: &mut Frame, area: Rect) {
        let provider_types: Vec<String> = SUPPORTED_MODEL_ENDPOINTS
            .iter()
            .map(|s| s.to_string())
            .collect();

        let items: Vec<ListItem> = provider_types
            .iter()
            .enumerate()
            .map(|(index, provider)| {
                let style = if provider == &self.provider_type {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Text::raw(format!("{}: {}", index + 1, provider)))
                    .style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Provider Type"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(
            provider_types
                .iter()
                .position(|p| p == &self.provider_type)
                .unwrap_or(0),
        ));

        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_select_model(&self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        if let Some(error_message) = &self.model_fetch_error {
            let available_width = area.width as usize - 4; // Subtract 4 for borders and padding
            let simple_string = SimpleString::from(error_message.clone());
            let wrapped_spans = simple_string.wrapped_spans(
                available_width,
                Some(Style::default().fg(Color::Red)),
                None,
            );
            for spans in wrapped_spans {
                items.push(ListItem::new(Line::from(spans)));
            }
        } else {
            items = self
                .available_models
                .iter()
                .enumerate()
                .map(|(index, model)| {
                    let style = if Some(index) == self.selected_model_index {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Text::raw(format!(
                        "{}: {}",
                        index + 1,
                        model.identifier.0
                    )))
                    .style(style)
                })
                .collect();
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Select Model"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(self.selected_model_index);

        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_configure_settings(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .additional_settings
            .iter()
            .map(|(key, setting)| {
                let (key_style, value_style) =
                    if Some(key) == self.current_setting_key.as_ref() {
                        (
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                            if self.is_editing {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::Yellow)
                            },
                        )
                    } else {
                        (Style::default(), Style::default())
                    };

                let key_content = format!("{}: ", setting.display_name);
                let value_content = if self.is_editing
                    && Some(key) == self.current_setting_key.as_ref()
                {
                    &self.edit_buffer
                } else {
                    &setting.value
                };

                let simple_string = SimpleString::from(format!(
                    "{}{}",
                    key_content, value_content
                ));
                let wrapped_spans = simple_string.wrapped_spans(
                    area.width as usize - 2,
                    Some(key_style),
                    None,
                );

                ListItem::new(
                    wrapped_spans
                        .into_iter()
                        .map(|spans| {
                            let mut line_spans = Vec::new();
                            for (i, span) in spans.into_iter().enumerate() {
                                if i == 0 && span.content == key_content {
                                    line_spans.push(span);
                                } else {
                                    line_spans.push(Span::styled(
                                        span.content,
                                        value_style,
                                    ));
                                }
                            }
                            Line::from(line_spans)
                        })
                        .collect::<Vec<Line>>(),
                )
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configure Settings"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(self.current_setting_key.as_ref().and_then(|key| {
            self.additional_settings.keys().position(|k| k == key)
        }));

        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_confirm(&self, f: &mut Frame, area: Rect) {
        let mut items = vec![
            ListItem::new(format!("Name: {}", self.name)),
            ListItem::new(format!("Provider Type: {}", self.provider_type)),
        ];

        if let Some(model) = &self.model_identifier {
            items.push(ListItem::new(format!("Model: {}", model)));
        }

        for (key, setting) in &self.additional_settings {
            items.push(ListItem::new(format!("{}: {}", key, setting.value)));
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm Provider Configuration"),
        );

        f.render_widget(list, area);
    }

    fn handle_enter_name(&mut self, input: KeyEvent) -> ProviderCreatorAction {
        match input.code {
            KeyCode::Char(c) => {
                self.name.push(c);
                ProviderCreatorAction::Refresh
            }
            KeyCode::Backspace => {
                self.name.pop();
                ProviderCreatorAction::Refresh
            }
            KeyCode::Enter => {
                if !self.name.is_empty() {
                    self.current_step =
                        ProviderCreationStep::SelectProviderType;
                    ProviderCreatorAction::Refresh
                } else {
                    ProviderCreatorAction::NoAction
                }
            }
            KeyCode::Esc => ProviderCreatorAction::Cancel,
            _ => ProviderCreatorAction::NoAction,
        }
    }

    async fn handle_select_provider_type(
        &mut self,
        input: KeyEvent,
    ) -> ProviderCreatorAction {
        let provider_types: Vec<String> = SUPPORTED_MODEL_ENDPOINTS
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Ensure the first item is selected by default if no selection has been made
        if self.provider_type.is_empty() && !provider_types.is_empty() {
            self.provider_type = provider_types[0].clone();
        }

        match input.code {
            KeyCode::Up => {
                let current_index = provider_types
                    .iter()
                    .position(|p| p == &self.provider_type)
                    .unwrap_or(0);
                if current_index > 0 {
                    self.provider_type =
                        provider_types[current_index - 1].clone();
                } else {
                    self.provider_type =
                        provider_types[provider_types.len() - 1].clone();
                }
                ProviderCreatorAction::Refresh
            }
            KeyCode::Down => {
                let current_index = provider_types
                    .iter()
                    .position(|p| p == &self.provider_type)
                    .unwrap_or(0);
                if current_index < provider_types.len() - 1 {
                    self.provider_type =
                        provider_types[current_index + 1].clone();
                } else {
                    self.provider_type = provider_types[0].clone();
                }
                ProviderCreatorAction::Refresh
            }
            KeyCode::Enter | KeyCode::Tab => {
                self.current_step = ProviderCreationStep::SelectModel;
                ProviderCreatorAction::LoadModels
            }
            KeyCode::Esc => {
                self.current_step = ProviderCreationStep::EnterName;
                ProviderCreatorAction::Refresh
            }
            _ => ProviderCreatorAction::NoAction,
        }
    }

    async fn handle_select_model(
        &mut self,
        input: KeyEvent,
    ) -> ProviderCreatorAction {
        match input.code {
            KeyCode::Up => {
                if let Some(index) = self.selected_model_index.as_mut() {
                    if *index > 0 {
                        *index -= 1;
                    } else {
                        *index = self.available_models.len() - 1;
                    }
                } else if !self.available_models.is_empty() {
                    self.selected_model_index =
                        Some(self.available_models.len() - 1);
                }
                ProviderCreatorAction::Refresh
            }
            KeyCode::Down => {
                if let Some(index) = self.selected_model_index.as_mut() {
                    if *index < self.available_models.len() - 1 {
                        *index += 1;
                    } else {
                        *index = 0;
                    }
                } else if !self.available_models.is_empty() {
                    self.selected_model_index = Some(0);
                }
                ProviderCreatorAction::Refresh
            }
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(index) = self.selected_model_index {
                    self.model_identifier =
                        Some(self.available_models[index].identifier.0.clone());
                    ProviderCreatorAction::LoadAdditionalSettings
                } else {
                    ProviderCreatorAction::NoAction
                }
            }
            KeyCode::Esc => {
                self.current_step = ProviderCreationStep::SelectProviderType;
                ProviderCreatorAction::Refresh
            }
            _ => ProviderCreatorAction::NoAction,
        }
    }

    fn handle_configure_settings(
        &mut self,
        input: KeyEvent,
    ) -> ProviderCreatorAction {
        match input.code {
            KeyCode::Up => {
                if !self.is_editing {
                    self.move_setting_selection(-1);
                }
                ProviderCreatorAction::Refresh
            }
            KeyCode::Down => {
                if !self.is_editing {
                    self.move_setting_selection(1);
                }
                ProviderCreatorAction::Refresh
            }
            KeyCode::Enter => {
                if self.is_editing {
                    self.save_current_setting();
                    self.is_editing = false;
                    if self.is_last_setting() {
                        self.current_step = ProviderCreationStep::Confirm;
                    } else {
                        self.move_setting_selection(1);
                    }
                } else {
                    self.start_editing_current_setting();
                }
                ProviderCreatorAction::Refresh
            }
            KeyCode::Esc => {
                if self.is_editing {
                    self.cancel_editing();
                } else {
                    self.current_step = ProviderCreationStep::SelectModel;
                }
                ProviderCreatorAction::Refresh
            }
            KeyCode::Tab => {
                if self.is_editing {
                    self.save_current_setting();
                }
                self.current_step = ProviderCreationStep::Confirm;
                ProviderCreatorAction::Refresh
            }
            KeyCode::Char(c) => {
                if !self.is_editing {
                    self.start_editing_current_setting();
                    self.edit_buffer.clear();
                }
                self.edit_buffer.push(c);
                ProviderCreatorAction::Refresh
            }
            KeyCode::Backspace => {
                if !self.is_editing {
                    self.start_editing_current_setting();
                    self.edit_buffer.clear();
                } else {
                    self.edit_buffer.pop();
                }
                ProviderCreatorAction::Refresh
            }
            _ => ProviderCreatorAction::NoAction,
        }
    }

    fn move_setting_selection(&mut self, delta: i32) {
        let keys: Vec<_> = self.additional_settings.keys().collect();
        let current_index = self
            .current_setting_key
            .as_ref()
            .and_then(|key| keys.iter().position(|&k| k == key))
            .unwrap_or(0);
        let new_index = (current_index as i32 + delta)
            .rem_euclid(keys.len() as i32) as usize;
        self.current_setting_key = Some(keys[new_index].clone());
    }

    fn start_editing_current_setting(&mut self) {
        if let Some(key) = &self.current_setting_key {
            if let Some(setting) = self.additional_settings.get(key) {
                self.edit_buffer = setting.value.clone();
                self.is_editing = true;
            }
        }
    }

    fn save_current_setting(&mut self) {
        if let Some(key) = &self.current_setting_key {
            if let Some(setting) = self.additional_settings.get_mut(key) {
                setting.value = self.edit_buffer.clone();
            }
        }
    }

    fn cancel_editing(&mut self) {
        self.is_editing = false;
        self.edit_buffer.clear();
    }

    fn is_last_setting(&self) -> bool {
        if let Some(current_key) = &self.current_setting_key {
            let keys: Vec<_> = self.additional_settings.keys().collect();
            keys.last().map(|&k| k) == Some(current_key)
        } else {
            false
        }
    }

    async fn handle_confirm(
        &mut self,
        input: KeyEvent,
    ) -> ProviderCreatorAction {
        match input.code {
            KeyCode::Enter => {
                match self.create_provider().await {
                    Ok(new_config) => ProviderCreatorAction::Finish(new_config),
                    Err(e) => {
                        // Handle the error appropriately, maybe set an error message to display
                        log::error!("Failed to create provider: {}", e);
                        ProviderCreatorAction::Refresh
                    }
                }
            }
            KeyCode::Esc => {
                if !self.additional_settings.is_empty() {
                    self.current_step = ProviderCreationStep::ConfigureSettings;
                } else {
                    self.current_step = ProviderCreationStep::SelectModel;
                }
                ProviderCreatorAction::Refresh
            }
            _ => ProviderCreatorAction::NoAction,
        }
    }

    pub async fn load_models(&mut self) -> Result<(), ApplicationError> {
        let model_server = ModelServer::from_str(&self.provider_type)?;
        match model_server.list_models().await {
            Ok(models) if !models.is_empty() => {
                self.available_models = models;
                self.selected_model_index = Some(0);
                self.model_fetch_error = None;
                Ok(())
            }
            Ok(_) | Err(ApplicationError::NotReady(_)) => {
                self.model_fetch_error = Some(
                    "Can't fetch models for this provider. Ensure the \
                     provider is running and correctly configured. Press Tab \
                     to skip setting a model."
                        .to_string(),
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn prepare_additional_settings(&mut self, model_server: &ModelServer) {
        self.additional_settings.clear();
        let additional_settings = model_server.get_profile_settings();
        if let JsonValue::Object(map) = additional_settings {
            for (key, value) in map {
                if !key.starts_with("__") {
                    if let JsonValue::Object(setting_map) = value {
                        let display_name = setting_map
                            .get("display_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&key)
                            .to_string();
                        let is_secure =
                            setting_map.get("encryption_key").is_some();
                        let placeholder = setting_map
                            .get("placeholder")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        self.additional_settings.insert(
                            key.clone(),
                            AdditionalSetting {
                                name: format!("__TEMPLATE.{}", key),
                                display_name,
                                value: String::new(),
                                is_secure,
                                placeholder,
                            },
                        );
                    }
                }
            }
        }
        if self.additional_settings.is_empty() {
            self.current_step = ProviderCreationStep::Confirm;
        } else {
            self.current_step = ProviderCreationStep::ConfigureSettings;
            self.current_setting_key =
                self.additional_settings.keys().next().cloned();
        }
    }

    async fn create_provider(
        &mut self,
    ) -> Result<ProviderConfig, ApplicationError> {
        let new_config = ProviderConfig {
            id: None,
            name: self.name.clone(),
            provider_type: self.provider_type.clone(),
            model_identifier: self.model_identifier.clone(),
            additional_settings: self.additional_settings.clone(),
        };

        _ = self.db_handler.save_provider_config(&new_config).await?;
        Ok(new_config)
    }
}

#[derive(Debug, Clone)]
pub enum ProviderCreatorAction {
    Refresh,
    LoadModels,
    LoadAdditionalSettings,
    Finish(ProviderConfig),
    Cancel,
    NoAction,
}

#[derive(Debug)]
pub enum ProviderManagerAction {
    Refresh,
    ProviderSelected,
    NoAction,
}
