use ratatui::layout::Margin;
use ratatui::text::Text;
use serde_json::{json, Value as JsonValue};

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderCreationStep {
    EnterName,
    SelectProviderType,
    SelectModel,
    ConfigureSettings,
    ConfirmCreate,
    CreatingProvider,
}

#[derive(Debug, Clone)]
pub struct ProviderCreator {
    name: String,
    provider_type: String,
    model_identifier: Option<String>,
    additional_settings: HashMap<String, ProviderConfigOptions>,
    db_handler: UserProfileDbHandler,
    pub current_step: ProviderCreationStep,
    available_models: Vec<ModelSpec>,
    current_setting_key: Option<String>,
    edit_buffer: String,
    is_editing: bool,
    model_fetch_error: Option<String>,
    text_area: Option<TextArea<ReadDocument>>,
    model_list: Option<ListWidget>,
    model_list_state: ListWidgetState,
}

impl ProviderCreator {
    pub async fn new(
        db_handler: UserProfileDbHandler,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            name: String::new(),
            provider_type: String::new(),
            model_identifier: None,
            additional_settings: HashMap::new(),
            db_handler,
            current_step: ProviderCreationStep::EnterName,
            available_models: Vec::new(),
            current_setting_key: None,
            edit_buffer: String::new(),
            is_editing: false,
            model_fetch_error: None,
            text_area: None,
            model_list: None,
            model_list_state: ListWidgetState::default(),
        })
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
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
            ProviderCreationStep::ConfirmCreate => {
                self.render_confirm_create(f, area)
            }
            ProviderCreationStep::CreatingProvider => {
                self.render_creating_provider(f, area)
            }
        }
    }

    pub fn render_confirm_create(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        let content_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(chunks[0]);

        let text_area_block = Block::default()
            .borders(Borders::ALL)
            .title("Provider Details");

        if let Some(text_area) = &mut self.text_area {
            text_area.render(f, content_area[0].inner(Margin::new(1, 1)));
        } else {
            let fallback_text =
                Paragraph::new("No provider details available.")
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

        let create_button = Paragraph::new("[ Create Provider ]")
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(create_button, button_chunks[1]);
    }

    fn initialize_confirm_create_state(&mut self) {
        let text_lines = self.create_confirm_details();
        self.text_area = Some(TextArea::with_read_document(Some(text_lines)));
    }

    fn create_confirm_details(&self) -> Vec<TextLine> {
        let mut lines = Vec::new();

        let mut name_line = TextLine::new();
        name_line.add_segment(
            "Name:",
            Some(Style::default().add_modifier(Modifier::BOLD)),
        );
        lines.push(name_line);

        let mut name_value_line = TextLine::new();
        name_value_line.add_segment(
            format!("  {}", self.name),
            Some(Style::default().fg(Color::Cyan)),
        );
        lines.push(name_value_line);

        lines.push(TextLine::new()); // Empty line for spacing

        // Provider Type
        let mut type_line = TextLine::new();
        type_line.add_segment(
            "Provider Type:",
            Some(Style::default().add_modifier(Modifier::BOLD)),
        );
        lines.push(type_line);

        let mut type_value_line = TextLine::new();
        type_value_line.add_segment(
            format!("  {}", self.provider_type),
            Some(Style::default().fg(Color::Cyan)),
        );
        lines.push(type_value_line);

        lines.push(TextLine::new()); // Empty line for spacing

        // Model
        if let Some(model) = &self.model_identifier {
            let mut model_line = TextLine::new();
            model_line.add_segment(
                "Model:",
                Some(Style::default().add_modifier(Modifier::BOLD)),
            );
            lines.push(model_line);

            let mut model_value_line = TextLine::new();
            model_value_line.add_segment(
                format!("  {}", model),
                Some(Style::default().fg(Color::Cyan)),
            );
            lines.push(model_value_line);

            lines.push(TextLine::new()); // Empty line for spacing
        }

        // Additional Settings
        if !self.additional_settings.is_empty() {
            let mut settings_line = TextLine::new();
            settings_line.add_segment(
                "Additional Settings:",
                Some(Style::default().add_modifier(Modifier::BOLD)),
            );
            lines.push(settings_line);

            for (key, setting) in &self.additional_settings {
                let mut setting_line = TextLine::new();
                setting_line.add_segment(
                    format!("  {}: ", key),
                    Some(Style::default().fg(Color::Yellow)),
                );
                setting_line.add_segment(
                    &setting.value,
                    Some(Style::default().fg(Color::Cyan)),
                );
                lines.push(setting_line);
            }
        }

        lines
    }

    pub async fn handle_key_event(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match input.code {
            KeyCode::Esc => {
                if self.current_step == ProviderCreationStep::ConfirmCreate {
                    self.text_area = None;
                }
                return self.go_to_previous_step();
            }
            KeyCode::Backspace => match self.current_step {
                ProviderCreationStep::EnterName if !self.name.is_empty() => {
                    self.name.pop();
                    return Ok(CreatorAction::Continue);
                }
                ProviderCreationStep::ConfirmCreate => {
                    self.text_area = None;
                    return self.go_to_previous_step();
                }
                _ => return self.go_to_previous_step(),
            },
            _ => {}
        }

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
            ProviderCreationStep::ConfirmCreate => {
                self.handle_confirm_create(input).await
            }
            ProviderCreationStep::CreatingProvider => {
                Ok(CreatorAction::Continue)
            }
        }
    }

    pub fn render_enter_name(&self, f: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.name.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Enter Provider Name"),
            );
        f.render_widget(input, area);
    }

    pub fn render_select_provider_type(&self, f: &mut Frame, area: Rect) {
        let provider_types: Vec<String> = SUPPORTED_MODEL_ENDPOINTS
            .iter()
            .map(|s| s.to_string())
            .collect();

        let items: Vec<ListItem> = provider_types
            .iter()
            .enumerate()
            .map(|(index, provider)| {
                let style = if provider == &self.provider_type {
                    Style::default().fg(Color::White)
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

    pub fn render_select_model(&mut self, f: &mut Frame, area: Rect) {
        if let Some(error_message) = &self.model_fetch_error {
            let mut error_text_area =
                TextArea::with_read_document(Some(vec![TextLine::from_text(
                    error_message,
                    Some(Style::default().fg(Color::Red)),
                )]));

            let block = Block::default().borders(Borders::ALL).title("Error");
            let inner_area = block.inner(area);
            f.render_widget(block, area);
            error_text_area.render(f, inner_area);
        } else if let Some(list_widget) = &self.model_list {
            f.render_stateful_widget(
                list_widget,
                area,
                &mut self.model_list_state,
            );
        }
    }

    pub fn render_configure_settings(&self, f: &mut Frame, area: Rect) {
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

    fn go_to_previous_step(
        &mut self,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match self.current_step {
            ProviderCreationStep::EnterName => Ok(CreatorAction::Cancel),
            ProviderCreationStep::SelectProviderType => {
                self.current_step = ProviderCreationStep::EnterName;
                Ok(CreatorAction::Continue)
            }
            ProviderCreationStep::SelectModel => {
                self.current_step = ProviderCreationStep::SelectProviderType;
                Ok(CreatorAction::Continue)
            }
            ProviderCreationStep::ConfigureSettings => {
                self.current_step = ProviderCreationStep::SelectModel;
                Ok(CreatorAction::Continue)
            }
            ProviderCreationStep::ConfirmCreate => {
                if self.additional_settings.is_empty() {
                    self.current_step = ProviderCreationStep::SelectModel;
                } else {
                    self.current_step = ProviderCreationStep::ConfigureSettings;
                }
                Ok(CreatorAction::Continue)
            }
            ProviderCreationStep::CreatingProvider => {
                self.current_step = ProviderCreationStep::ConfirmCreate;
                Ok(CreatorAction::Continue)
            }
        }
    }

    pub fn handle_enter_name(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match input.code {
            KeyCode::Char(c) => {
                self.name.push(c);
                Ok(CreatorAction::Continue)
            }
            KeyCode::Enter => {
                if !self.name.is_empty() {
                    self.current_step =
                        ProviderCreationStep::SelectProviderType;
                }
                Ok(CreatorAction::Continue)
            }
            _ => Ok(CreatorAction::Continue),
        }
    }

    async fn handle_select_provider_type(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        let provider_types: Vec<String> = SUPPORTED_MODEL_ENDPOINTS
            .iter()
            .map(|s| s.to_string())
            .collect();

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
            }
            KeyCode::Enter => {
                if !self.provider_type.is_empty() {
                    self.current_step = ProviderCreationStep::SelectModel;
                    self.load_models().await?;
                }
            }
            _ => {}
        }
        Ok(CreatorAction::Continue)
    }

    async fn handle_select_model(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match input.code {
            KeyCode::Up => {
                if let Some(list_widget) = &self.model_list {
                    list_widget.move_selection(&mut self.model_list_state, -1);
                }
            }
            KeyCode::Down => {
                if let Some(list_widget) = &self.model_list {
                    list_widget.move_selection(&mut self.model_list_state, 1);
                }
            }
            KeyCode::Enter => {
                if let Some(list_widget) = &self.model_list {
                    if let Some(model_name) = list_widget
                        .get_selected_item_content(&self.model_list_state)
                    {
                        self.model_identifier = Some(model_name);
                        let model_server =
                            ModelServer::from_str(&self.provider_type)?;
                        self.prepare_additional_settings(&model_server);
                        return Ok(CreatorAction::LoadAdditionalSettings);
                    }
                }
            }
            KeyCode::Tab => {
                // Skip model selection if there's an error or no models available
                if self.model_fetch_error.is_some() || self.model_list.is_none()
                {
                    let model_server =
                        ModelServer::from_str(&self.provider_type)?;
                    self.prepare_additional_settings(&model_server);
                    return Ok(CreatorAction::LoadAdditionalSettings);
                }
            }
            _ => {}
        }
        Ok(CreatorAction::Continue)
    }

    fn handle_configure_settings(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match input.code {
            KeyCode::Up => {
                if !self.is_editing {
                    self.move_setting_selection(-1);
                }
            }
            KeyCode::Down => {
                if !self.is_editing {
                    self.move_setting_selection(1);
                }
            }
            KeyCode::Enter => {
                if self.is_editing {
                    self.save_current_setting();
                    self.is_editing = false;
                    if self.is_last_setting() {
                        self.current_step = ProviderCreationStep::ConfirmCreate;
                        self.initialize_confirm_create_state();
                        return Ok(CreatorAction::Continue);
                    } else {
                        self.move_setting_selection(1);
                    }
                } else {
                    self.start_editing_current_setting();
                }
            }
            KeyCode::Tab => {
                if self.is_editing {
                    self.save_current_setting();
                }
                self.current_step = ProviderCreationStep::ConfirmCreate;
                self.initialize_confirm_create_state();
                return Ok(CreatorAction::Continue);
            }
            KeyCode::Char(c) => {
                if !self.is_editing {
                    self.start_editing_current_setting();
                    self.edit_buffer.clear();
                }
                self.edit_buffer.push(c);
            }
            KeyCode::Backspace => {
                if self.is_editing {
                    self.edit_buffer.pop();
                } else {
                    return self.go_to_previous_step();
                }
            }
            _ => {}
        }
        Ok(CreatorAction::Continue)
    }

    fn move_setting_selection(&mut self, delta: i32) {
        let keys: Vec<_> = self.additional_settings.keys().collect();
        if keys.is_empty() {
            // No settings to select, so we can't move the selection
            return;
        }

        let current_index = self
            .current_setting_key
            .as_ref()
            .and_then(|key| keys.iter().position(|&k| k == key))
            .unwrap_or(0);

        let keys_len = keys.len() as i32;
        let new_index =
            (((current_index as i32 + delta) % keys_len) + keys_len) % keys_len;

        self.current_setting_key = Some(keys[new_index as usize].clone());
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

    fn is_last_setting(&self) -> bool {
        if let Some(current_key) = &self.current_setting_key {
            let keys: Vec<_> = self.additional_settings.keys().collect();
            keys.last().map(|&k| k) == Some(current_key)
        } else {
            false
        }
    }

    async fn handle_confirm_create(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match input.code {
            KeyCode::Enter => {
                self.current_step = ProviderCreationStep::CreatingProvider;
                match self.create_provider().await {
                    Ok(new_config) => {
                        // Immediately return the Finish action with the new config
                        Ok(CreatorAction::Finish(new_config))
                    }
                    Err(e) => {
                        log::error!("Failed to create provider: {}", e);
                        self.current_step = ProviderCreationStep::ConfirmCreate;
                        Ok(CreatorAction::Continue)
                    }
                }
            }
            KeyCode::Esc => {
                self.text_area = None;
                self.go_to_previous_step()
            }
            _ => {
                // Forward other key events to the TextArea
                if let Some(text_area) = &mut self.text_area {
                    text_area.handle_key_event(input);
                }
                Ok(CreatorAction::Continue)
            }
        }
    }

    pub async fn load_models(&mut self) -> Result<(), ApplicationError> {
        let model_server = ModelServer::from_str(&self.provider_type)?;
        match model_server.list_models().await {
            Ok(models) if !models.is_empty() => {
                self.available_models = models;
                let model_items: Vec<Text<'static>> = self
                    .available_models
                    .iter()
                    .map(|model| {
                        Text::from(Line::from(Span::raw(
                            model.identifier.0.clone(),
                        )))
                    })
                    .collect();
                self.model_list = Some(
                    ListWidget::new(model_items)
                        .title("Select Model")
                        .normal_style(Style::default().fg(Color::Cyan))
                        .selected_style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol(">> ".to_string()),
                );
                self.model_list_state = ListWidgetState::default();
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
                            ProviderConfigOptions {
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
            self.current_step = ProviderCreationStep::ConfirmCreate;
        } else {
            self.current_step = ProviderCreationStep::ConfigureSettings;
            self.current_setting_key =
                self.additional_settings.keys().next().cloned();
        }
    }

    pub async fn create_provider(
        &mut self,
    ) -> Result<ConfigItem, ApplicationError> {
        let new_config = self
            .db_handler
            .create_configuration_item(
                self.name.clone(),
                "provider",
                self.create_provider_parameters(),
            )
            .await?;

        Ok(ConfigItem::DatabaseConfig(new_config))
    }

    fn create_provider_parameters(&self) -> serde_json::Value {
        json!({
            "provider_type": self.provider_type,
            "model_identifier": self.model_identifier,
            "additional_settings": self.additional_settings,
        })
    }

    pub fn render_creating_provider(&self, f: &mut Frame, area: Rect) {
        let content = format!("Creating provider '{}'...", self.name);

        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Creating Provider"),
            );

        f.render_widget(paragraph, area);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderConfigOptions {
    pub name: String,
    pub display_name: String,
    pub value: String,
    pub is_secure: bool,
    pub placeholder: String,
}
