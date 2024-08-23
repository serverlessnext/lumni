use std::collections::{HashMap, VecDeque};

use super::*;

#[derive(Debug)]
pub enum BackgroundTaskResult {
    ProfileCreated(Result<UserProfile, ApplicationError>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NewProfileCreationStep {
    EnterName,
    SelectProfileType,
    SelectModel,
    InputAdditionalSettings,
    ConfirmCreate,
    CreatingProfile,
}

#[derive(Debug, Clone)]
pub enum NewProfileCreatorAction {
    Refresh,
    WaitForKeyEvent,
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionState {
    ProfileType(usize),
    ModelOption(usize),
    AdditionalSetting(usize),
    NextButton,
    CreateButton,
}

#[derive(Debug, Clone)]
pub struct SubSelection {
    options: Vec<String>,
    selected: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct AdditionalSetting {
    name: String,
    display_name: String,
    value: String,
    is_secure: bool,
    placeholder: String,
}

#[derive(Debug)]
pub struct NewProfileCreator {
    pub predefined_types: Vec<String>,
    pub selected_type_index: usize,
    pub new_profile_name: String,
    pub creation_step: NewProfileCreationStep,
    pub background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    pub task_start_time: Option<Instant>,
    pub db_handler: UserProfileDbHandler,
    pub sub_selections: HashMap<String, Vec<SubSelection>>,
    pub current_sub_selection: Option<usize>,
    pub selection_state: SelectionState,
    pub ready_to_create: bool,
    pub skipped_type_selection: bool,
    pub previous_step: Option<NewProfileCreationStep>,
    navigation_stack: VecDeque<NewProfileCreationStep>,
    additional_settings: Vec<AdditionalSetting>,
    current_additional_setting: usize,
    is_input_focused: bool,
    temp_input: Option<String>,
}

impl NewProfileCreator {
    const MIN_INPUT_WIDTH: usize = 20;
    const MAX_INPUT_WIDTH: usize = 32;
    const COLOR_BACKGROUND: Color = Color::Rgb(16, 24, 32); // Dark blue-gray
    const COLOR_FOREGROUND: Color = Color::Rgb(220, 220, 220); // Light gray
    const COLOR_HIGHLIGHT: Color = Color::Rgb(52, 152, 219); // Bright blue
    const COLOR_SECONDARY: Color = Color::Rgb(241, 196, 15); // Yellow
    const COLOR_SUCCESS: Color = Color::Rgb(46, 204, 113); // Green

    pub fn new(db_handler: UserProfileDbHandler) -> Self {
        let predefined_types: Vec<String> = SUPPORTED_MODEL_ENDPOINTS
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut sub_selections = HashMap::new();
        for profile_type in &predefined_types {
            sub_selections.insert(
                profile_type.clone(),
                vec![SubSelection {
                    options: vec![],
                    selected: None,
                }],
            );
        }

        let mut creator = Self {
            predefined_types,
            selected_type_index: 0,
            new_profile_name: String::new(),
            creation_step: NewProfileCreationStep::EnterName,
            background_task: None,
            task_start_time: None,
            db_handler,
            sub_selections,
            current_sub_selection: None,
            selection_state: SelectionState::ProfileType(0),
            skipped_type_selection: false,
            ready_to_create: false,
            previous_step: None,
            navigation_stack: VecDeque::new(),
            additional_settings: Vec::new(),
            current_additional_setting: 0,
            is_input_focused: false,
            temp_input: None,
        };
        creator
            .navigation_stack
            .push_back(NewProfileCreationStep::EnterName);
        creator
    }

    pub async fn handle_key_event(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.creation_step {
            NewProfileCreationStep::EnterName => {
                self.handle_enter_name(key_code)
            }
            _ => match key_code {
                KeyCode::Esc => self.handle_back_navigation(),
                _ => match self.creation_step {
                    NewProfileCreationStep::SelectProfileType => {
                        self.handle_select_profile_type(key_code).await
                    }
                    NewProfileCreationStep::SelectModel => {
                        self.handle_select_model(key_code)
                    }
                    NewProfileCreationStep::InputAdditionalSettings => {
                        self.handle_input_additional_settings(key_code)
                    }
                    NewProfileCreationStep::ConfirmCreate => {
                        self.handle_confirm_create(key_code).await
                    }
                    NewProfileCreationStep::CreatingProfile => {
                        Ok(NewProfileCreatorAction::WaitForKeyEvent)
                    }
                    _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
                },
            },
        }
    }

    fn handle_back_navigation(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.creation_step {
            NewProfileCreationStep::SelectModel => {
                match self.selection_state {
                    SelectionState::NextButton => {
                        // Go back to the previously selected model option
                        if let Some(profile_type) =
                            self.predefined_types.get(self.selected_type_index)
                        {
                            if let Some(sub_selections) =
                                self.sub_selections.get(profile_type)
                            {
                                if let Some(model_selection) =
                                    sub_selections.first()
                                {
                                    if let Some(selected_index) =
                                        model_selection.selected
                                    {
                                        self.selection_state =
                                            SelectionState::ModelOption(
                                                selected_index,
                                            );
                                        return Ok(
                                            NewProfileCreatorAction::Refresh,
                                        );
                                    }
                                }
                            }
                        }
                        // If we can't find the previously selected model, fall back to the first model
                        self.selection_state = SelectionState::ModelOption(0);
                        Ok(NewProfileCreatorAction::Refresh)
                    }
                    _ => {
                        // Go back to provider type selection
                        self.creation_step =
                            NewProfileCreationStep::SelectProfileType;
                        self.selection_state = SelectionState::ProfileType(
                            self.selected_type_index,
                        );
                        self.navigation_stack.pop_back();
                        Ok(NewProfileCreatorAction::Refresh)
                    }
                }
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                if self.is_input_focused {
                    // If an input is focused, just unfocus it
                    self.is_input_focused = false;
                    self.temp_input = None;
                    Ok(NewProfileCreatorAction::Refresh)
                } else {
                    // Move to the previous item or step
                    self.move_to_previous_item()
                }
            }
            _ => {
                if self.navigation_stack.len() > 1 {
                    self.navigation_stack.pop_back();
                    let previous_step = self
                        .navigation_stack
                        .back()
                        .cloned()
                        .unwrap_or(NewProfileCreationStep::EnterName);
                    self.creation_step = previous_step;
                    self.reset_step_state();
                    Ok(NewProfileCreatorAction::Refresh)
                } else {
                    Ok(NewProfileCreatorAction::Cancel)
                }
            }
        }
    }

    fn reset_step_state(&mut self) {
        match self.creation_step {
            NewProfileCreationStep::EnterName => {
                self.skipped_type_selection = false;
                self.ready_to_create = false;
            }
            NewProfileCreationStep::SelectProfileType => {
                self.skipped_type_selection = false;
                self.ready_to_create = false;
                self.selection_state =
                    SelectionState::ProfileType(self.selected_type_index);
            }
            NewProfileCreationStep::SelectModel => {
                self.selection_state = SelectionState::ModelOption(0);
                self.ready_to_create = false;
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                self.selection_state = SelectionState::AdditionalSetting(0);
                self.ready_to_create = false;
            }
            NewProfileCreationStep::ConfirmCreate => {
                self.selection_state = SelectionState::CreateButton;
                self.ready_to_create = true;
            }
            NewProfileCreationStep::CreatingProfile => {
                // This state shouldn't be reached through normal navigation
            }
        }
    }

    fn handle_enter_name(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
            KeyCode::Char(c) => {
                self.new_profile_name.push(c);
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Backspace => {
                self.new_profile_name.pop();
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Enter => {
                if !self.new_profile_name.is_empty() {
                    self.selection_state = SelectionState::ProfileType(0);
                    self.selected_type_index = 0;
                    self.move_to_next_step(
                        NewProfileCreationStep::SelectProfileType,
                    )
                } else {
                    Ok(NewProfileCreatorAction::WaitForKeyEvent)
                }
            }
            KeyCode::Esc => self.handle_back_navigation(),
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    async fn handle_select_profile_type(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
            KeyCode::Char('q') => self.handle_back_navigation(),
            KeyCode::Up => {
                if !self.skipped_type_selection {
                    if let SelectionState::ProfileType(index) =
                        &mut self.selection_state
                    {
                        if *index > 0 {
                            *index -= 1;
                            self.selected_type_index = *index;
                        }
                    }
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Down => {
                if !self.skipped_type_selection {
                    if let SelectionState::ProfileType(index) =
                        &mut self.selection_state
                    {
                        if *index < self.predefined_types.len() - 1 {
                            *index += 1;
                            self.selected_type_index = *index;
                        }
                    }
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Enter => {
                if self.skipped_type_selection {
                    self.ready_to_create = true;
                    self.move_to_next_step(
                        NewProfileCreationStep::ConfirmCreate,
                    )
                } else {
                    self.selected_type_index = self.get_selected_type_index();
                    self.prepare_for_model_selection().await
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.skipped_type_selection = !self.skipped_type_selection;
                self.ready_to_create = self.skipped_type_selection;
                if self.skipped_type_selection {
                    self.selection_state = SelectionState::CreateButton;
                } else {
                    self.selection_state =
                        SelectionState::ProfileType(self.selected_type_index);
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    fn update_selected_model(&mut self, index: usize) {
        if let Some(profile_type) =
            self.predefined_types.get(self.selected_type_index)
        {
            if let Some(sub_selections) =
                self.sub_selections.get_mut(profile_type)
            {
                if let Some(model_selection) = sub_selections.first_mut() {
                    model_selection.selected = Some(index);
                }
            }
        }
    }

    fn move_to_next_step(
        &mut self,
        next_step: NewProfileCreationStep,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        self.creation_step = next_step.clone();
        self.navigation_stack.push_back(next_step);
        self.reset_step_state();
        Ok(NewProfileCreatorAction::Refresh)
    }

    pub fn get_instructions(&self) -> &'static str {
        match self.creation_step {
            NewProfileCreationStep::EnterName => {
                "Enter profile name | Enter: Confirm | Esc: Cancel"
            }
            NewProfileCreationStep::SelectProfileType => {
                "↑↓: Select Type | Enter: Confirm | Esc: Back"
            }
            NewProfileCreationStep::SelectModel => {
                "↑↓: Select Model | Enter: Confirm | Esc: Back to Provider \
                 selection"
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                "Enter value for each setting | Enter: Next/Confirm | Esc: Back"
            }
            NewProfileCreationStep::ConfirmCreate => {
                "Enter: Create Profile | Esc: Back to Additional Settings"
            }
            NewProfileCreationStep::CreatingProfile => "Creating profile...",
        }
    }

    fn prepare_additional_settings(&mut self, model_server: &ModelServer) {
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
                        self.additional_settings.push(AdditionalSetting {
                            name: format!("__TEMPLATE.{}", key),
                            display_name,
                            value: String::new(),
                            is_secure,
                            placeholder,
                        });
                    }
                }
            }
        }
        self.current_additional_setting = 0;
    }

    pub async fn prepare_for_model_selection(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        let selected_type_index = self.get_selected_type_index();
        if let Some(profile_type) =
            self.predefined_types.get(selected_type_index).cloned()
        {
            let model_server = ModelServer::from_str(&profile_type)?;

            match model_server.list_models().await {
                Ok(models) if !models.is_empty() => {
                    let model_options: Vec<String> =
                        models.iter().map(|m| m.identifier.0.clone()).collect();

                    // Update the existing SubSelection for models
                    if let Some(sub_selections) =
                        self.sub_selections.get_mut(&profile_type)
                    {
                        if let Some(model_selection) =
                            sub_selections.first_mut()
                        {
                            model_selection.options = model_options;
                            model_selection.selected = Some(0);
                        }
                    }

                    // Prepare additional settings
                    self.prepare_additional_settings(&model_server);

                    // Set the selection state to ModelOption(0)
                    self.selection_state = SelectionState::ModelOption(0);
                    self.ready_to_create = false;

                    self.move_to_next_step(NewProfileCreationStep::SelectModel)
                }
                Ok(_) => Err(ApplicationError::NotReady(
                    "No models available for this server. Please try another \
                     provider."
                        .to_string(),
                )),
                Err(ApplicationError::NotReady(msg)) => {
                    Err(ApplicationError::NotReady(msg))
                }
                Err(e) => Err(e),
            }
        } else {
            Err(ApplicationError::NotReady(
                "Invalid provider selected.".to_string(),
            ))
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Name input
                Constraint::Min(1), // Type and sub-option selection or profile creation
            ])
            .split(area);

        self.render_profile_name_input(f, chunks[0]);

        match self.creation_step {
            NewProfileCreationStep::EnterName => {
                // No need to render anything else
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                self.render_additional_settings(f, chunks[1]);
            }
            NewProfileCreationStep::SelectProfileType
            | NewProfileCreationStep::SelectModel
            | NewProfileCreationStep::ConfirmCreate => {
                self.render_type_and_sub_options(f, chunks[1]);
            }
            NewProfileCreationStep::CreatingProfile => {
                self.render_creating_profile(f, chunks[1]);
            }
        }
    }

    fn render_profile_name_input(&self, f: &mut Frame, area: Rect) {
        let name_color =
            if self.creation_step == NewProfileCreationStep::EnterName {
                Self::COLOR_HIGHLIGHT
            } else {
                Self::COLOR_FOREGROUND
            };

        let input = Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                &self.new_profile_name,
                Style::default().fg(name_color),
            ),
        ]))
        .style(Style::default().fg(Self::COLOR_FOREGROUND))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(
                    if self.creation_step == NewProfileCreationStep::EnterName {
                        Self::COLOR_HIGHLIGHT
                    } else {
                        Self::COLOR_FOREGROUND
                    },
                ))
                .title("Enter New Profile Name"),
        );
        f.render_widget(input, area);
    }

    fn render_type_and_sub_options(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        match self.creation_step {
            NewProfileCreationStep::SelectProfileType => {
                self.render_profile_type_selection(f, chunks[0]);
            }
            NewProfileCreationStep::SelectModel => {
                self.render_model_selection(f, chunks[0]);
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                self.render_additional_settings(f, chunks[0]);
            }
            NewProfileCreationStep::ConfirmCreate => {
                self.render_confirmation(f, area);
            }
            _ => {}
        }

        // Render the Next/Create button for all steps
        self.render_next_or_create_button(f, chunks[1]);
    }

    fn render_profile_type_selection(&self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        if self.skipped_type_selection {
            let skipped_message =
                SimpleString::from("Provider selection skipped");
            let wrapped_spans = skipped_message.wrapped_spans(
                area.width as usize - 4,
                Some(
                    Style::default()
                        .fg(Self::COLOR_SECONDARY)
                        .add_modifier(Modifier::ITALIC),
                ),
            );
            for spans in wrapped_spans {
                items.push(ListItem::new(Line::from(spans)));
            }

            items.push(ListItem::new(""));
            let ready_message = SimpleString::from(
                "Press 'S' to undo skip and select a provider",
            );
            let wrapped_spans = ready_message.wrapped_spans(
                area.width as usize - 4,
                Some(Style::default().fg(Self::COLOR_HIGHLIGHT)),
            );
            for spans in wrapped_spans {
                items.push(ListItem::new(Line::from(spans)));
            }
        } else {
            for (i, profile_type) in self.predefined_types.iter().enumerate() {
                let style = if matches!(self.selection_state, SelectionState::ProfileType(selected) if selected == i)
                {
                    Style::default()
                        .fg(Self::COLOR_HIGHLIGHT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Self::COLOR_FOREGROUND)
                };

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", if matches!(self.selection_state, SelectionState::ProfileType(selected) if selected == i) { ">" } else { " " }),
                        Style::default().fg(Self::COLOR_SECONDARY),
                    ),
                    Span::styled(profile_type, style),
                ])));
            }

            items.push(ListItem::new(""));
            let ready_message = SimpleString::from(
                "Press 'S' to undo skip and select a provider",
            );
            let wrapped_spans = ready_message.wrapped_spans(
                area.width as usize - 4,
                Some(Style::default().fg(Self::COLOR_SECONDARY)),
            );
            for spans in wrapped_spans {
                items.push(ListItem::new(Line::from(spans)));
            }
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Provider"),
        );

        f.render_widget(list, area);
    }

    fn render_additional_settings(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        let available_width = chunks[0].width as usize - 3; // Subtract 3 for the left border and spacing

        let mut items = Vec::new();
        for (index, setting) in self.additional_settings.iter().enumerate() {
            let is_selected = matches!(self.selection_state, SelectionState::AdditionalSetting(selected) if selected == index);
            let is_focused = is_selected && self.is_input_focused;

            let label_style = if is_selected {
                Style::default()
                    .fg(Self::COLOR_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Self::COLOR_FOREGROUND)
            };

            let input_style = Style::default()
                .fg(if is_focused {
                    Self::COLOR_HIGHLIGHT
                } else {
                    Self::COLOR_FOREGROUND
                })
                .bg(if is_focused {
                    Color::DarkGray
                } else {
                    Self::COLOR_BACKGROUND
                });

            let arrow = if is_selected { ">" } else { " " };

            let label = format!("{}: ", setting.display_name);
            let label_width = label.chars().count();

            let input_width = (available_width - label_width)
                .max(Self::MIN_INPUT_WIDTH)
                .min(Self::MAX_INPUT_WIDTH);

            let mut line = vec![
                Span::styled(arrow, Style::default().fg(Self::COLOR_SECONDARY)),
                Span::raw(" "),
                Span::styled(label, label_style),
            ];

            // Create the input field
            let input_content = if self.is_input_focused
                && self.selection_state
                    == SelectionState::AdditionalSetting(index)
            {
                self.temp_input.as_ref().unwrap_or(&setting.value)
            } else if setting.value.is_empty() && !is_focused {
                &setting.placeholder
            } else {
                &setting.value
            };

            let input_field =
                format!("{:width$}", input_content, width = input_width)
                    .chars()
                    .take(input_width)
                    .collect::<String>();

            line.push(Span::styled(input_field, input_style));

            items.push(ListItem::new(Line::from(line)));
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Additional Settings"),
        );

        f.render_widget(list, chunks[0]);

        // Render the Next button
        let next_button = Paragraph::new("Next")
            .style(Style::default().fg(Self::COLOR_HIGHLIGHT).add_modifier(
                if matches!(self.selection_state, SelectionState::NextButton) {
                    Modifier::REVERSED
                } else {
                    Modifier::empty()
                },
            ))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(next_button, chunks[1]);
    }

    fn render_next_or_create_button(&self, f: &mut Frame, area: Rect) {
        let (button_text, button_style, is_selected) = match self.creation_step
        {
            NewProfileCreationStep::ConfirmCreate => {
                ("Create", Style::default().fg(Self::COLOR_SUCCESS), true)
            }
            _ => (
                "Next",
                Style::default().fg(Self::COLOR_HIGHLIGHT),
                matches!(self.selection_state, SelectionState::NextButton),
            ),
        };

        let button = Paragraph::new(button_text)
            .style(if is_selected {
                button_style.add_modifier(Modifier::REVERSED)
            } else {
                button_style
            })
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(button, area);
    }

    fn get_selected_model(&self) -> Option<(usize, String)> {
        if let Some(profile_type) =
            self.predefined_types.get(self.selected_type_index)
        {
            if let Some(sub_selections) = self.sub_selections.get(profile_type)
            {
                if let Some(model_selection) = sub_selections.first() {
                    if let Some(selected_index) = model_selection.selected {
                        return model_selection
                            .options
                            .get(selected_index)
                            .map(|model| (selected_index, model.clone()));
                    }
                }
            }
        }
        None
    }

    fn handle_select_model(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
            KeyCode::Up => self.move_model_selection_up(),
            KeyCode::Down => self.move_model_selection_down(),
            KeyCode::Enter => self.confirm_model_selection(),
            KeyCode::Tab => self.toggle_next_button(),
            KeyCode::Char('q') => self.handle_back_navigation(),
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    fn toggle_next_button(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.selection_state {
            SelectionState::ModelOption(_) => {
                self.selection_state = SelectionState::NextButton;
                Ok(NewProfileCreatorAction::Refresh)
            }
            SelectionState::AdditionalSetting(index) => {
                self.current_additional_setting = index;
                self.selection_state = SelectionState::NextButton;
                Ok(NewProfileCreatorAction::Refresh)
            }
            SelectionState::NextButton => {
                match self.creation_step {
                    NewProfileCreationStep::SelectModel => {
                        // Go back to the previously selected model option
                        if let Some(profile_type) =
                            self.predefined_types.get(self.selected_type_index)
                        {
                            if let Some(sub_selections) =
                                self.sub_selections.get(profile_type)
                            {
                                if let Some(model_selection) =
                                    sub_selections.first()
                                {
                                    if let Some(selected_index) =
                                        model_selection.selected
                                    {
                                        self.selection_state =
                                            SelectionState::ModelOption(
                                                selected_index,
                                            );
                                        return Ok(
                                            NewProfileCreatorAction::Refresh,
                                        );
                                    }
                                }
                            }
                        }
                        // If we can't find the previously selected model, fall back to the first model
                        self.selection_state = SelectionState::ModelOption(0);
                    }
                    NewProfileCreationStep::InputAdditionalSettings => {
                        if !self.additional_settings.is_empty() {
                            self.selection_state =
                                SelectionState::AdditionalSetting(
                                    self.current_additional_setting,
                                );
                        }
                    }
                    _ => {}
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    fn move_model_selection_up(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.selection_state {
            SelectionState::ModelOption(index) if index > 0 => {
                self.selection_state = SelectionState::ModelOption(index - 1);
            }
            SelectionState::NextButton => {
                let max_index = self.get_max_model_option_index();
                if max_index > 0 {
                    self.selection_state =
                        SelectionState::ModelOption(max_index - 1);
                }
            }
            _ => {}
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn move_model_selection_down(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.selection_state {
            SelectionState::ModelOption(index) => {
                let max_index = self.get_max_model_option_index();
                if index < max_index - 1 {
                    self.selection_state =
                        SelectionState::ModelOption(index + 1);
                }
            }
            _ => {}
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn move_to_next_button(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::ModelOption(index) = self.selection_state {
            self.update_selected_model(index);
        }
        self.selection_state = SelectionState::NextButton;
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn get_selected_type_index(&self) -> usize {
        match self.selection_state {
            SelectionState::ProfileType(index) => index,
            _ => self.selected_type_index,
        }
    }

    fn get_max_model_option_index(&self) -> usize {
        let selected_type_index = self.get_selected_type_index();
        self.predefined_types
            .get(selected_type_index)
            .and_then(|profile_type| self.sub_selections.get(profile_type))
            .and_then(|sub_selections| sub_selections.first())
            .map(|sub_selection| sub_selection.options.len())
            .unwrap_or(0)
    }

    fn handle_input_additional_settings(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
            KeyCode::Up => self.move_additional_setting_up(),
            KeyCode::Down => self.move_additional_setting_down(),
            KeyCode::Right => self.handle_right_arrow_additional_setting(),
            KeyCode::Enter => self.handle_enter_additional_setting(),
            KeyCode::Backspace => self.handle_backspace_additional_setting(),
            KeyCode::Delete => self.handle_delete_additional_setting(),
            KeyCode::Char(c) => self.handle_char_additional_setting(c),
            KeyCode::Tab => self.toggle_next_button(),
            KeyCode::Esc => self.handle_esc_additional_setting(),
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    fn handle_enter_additional_setting(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.selection_state {
            SelectionState::AdditionalSetting(current_index) => {
                if !self.is_input_focused {
                    // Start editing
                    self.temp_input = Some(
                        self.additional_settings[current_index].value.clone(),
                    );
                    self.is_input_focused = true;
                } else {
                    // Finish editing
                    if let Some(temp) = self.temp_input.take() {
                        if temp == self.additional_settings[current_index].value
                        {
                            // If no changes were made, move to next item
                            self.is_input_focused = false;
                            self.move_to_next_item(current_index);
                        } else {
                            // Apply changes and move to next item
                            self.additional_settings[current_index].value =
                                temp;
                            self.is_input_focused = false;
                            self.move_to_next_item(current_index);
                        }
                    }
                }
            }
            SelectionState::NextButton => {
                // Move to the confirmation/create window
                self.ready_to_create = true;
                self.move_to_next_step(NewProfileCreationStep::ConfirmCreate)?;
            }
            _ => {}
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn move_to_previous_item(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.selection_state {
            SelectionState::AdditionalSetting(current_index) => {
                if current_index > 0 {
                    // Move to the previous additional setting
                    self.selection_state =
                        SelectionState::AdditionalSetting(current_index - 1);
                    Ok(NewProfileCreatorAction::Refresh)
                } else {
                    // If at the first additional setting, go back to model selection
                    self.creation_step = NewProfileCreationStep::SelectModel;
                    if let Some(profile_type) =
                        self.predefined_types.get(self.selected_type_index)
                    {
                        if let Some(sub_selections) =
                            self.sub_selections.get(profile_type)
                        {
                            if let Some(model_selection) =
                                sub_selections.first()
                            {
                                if let Some(selected_index) =
                                    model_selection.selected
                                {
                                    self.selection_state =
                                        SelectionState::ModelOption(
                                            selected_index,
                                        );
                                }
                            }
                        }
                    }
                    self.navigation_stack.pop_back();
                    Ok(NewProfileCreatorAction::Refresh)
                }
            }
            SelectionState::NextButton => {
                // Move to the last additional setting
                if !self.additional_settings.is_empty() {
                    self.selection_state = SelectionState::AdditionalSetting(
                        self.additional_settings.len() - 1,
                    );
                    Ok(NewProfileCreatorAction::Refresh)
                } else {
                    // If no additional settings, go back to model selection
                    self.creation_step = NewProfileCreationStep::SelectModel;
                    // ... (same logic as above for setting model selection state)
                    self.navigation_stack.pop_back();
                    Ok(NewProfileCreatorAction::Refresh)
                }
            }
            _ => {
                // This shouldn't happen in the additional settings step, but just in case
                Ok(NewProfileCreatorAction::WaitForKeyEvent)
            }
        }
    }

    fn move_to_next_item(&mut self, current_index: usize) {
        if current_index + 1 < self.additional_settings.len() {
            // Move to the next additional setting
            self.selection_state =
                SelectionState::AdditionalSetting(current_index + 1);
        } else {
            // Move to the Next button if there are no more additional settings
            self.selection_state = SelectionState::NextButton;
        }
    }

    fn handle_right_arrow_additional_setting(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::AdditionalSetting(index) = self.selection_state {
            if !self.is_input_focused {
                // Focus the input and prepare for appending
                self.temp_input =
                    Some(self.additional_settings[index].value.clone());
                self.is_input_focused = true;
            }
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn handle_char_additional_setting(
        &mut self,
        c: char,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::AdditionalSetting(_) = self.selection_state {
            if !self.is_input_focused {
                // Start new input
                self.temp_input = Some(String::new());
                self.is_input_focused = true;
            }
            if let Some(temp) = self.temp_input.as_mut() {
                temp.push(c);
            }
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn handle_backspace_additional_setting(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::AdditionalSetting(index) = self.selection_state {
            if self.is_input_focused {
                if let Some(temp) = self.temp_input.as_mut() {
                    temp.pop();
                }
            } else {
                // If not focused, clear the selected item
                self.additional_settings[index].value.clear();
            }
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn handle_delete_additional_setting(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::AdditionalSetting(index) = self.selection_state {
            if self.is_input_focused {
                // Clear the temporary input
                self.temp_input = Some(String::new());
            } else {
                // Clear the selected item
                self.additional_settings[index].value.clear();
            }
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn handle_esc_additional_setting(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if self.is_input_focused {
            self.temp_input = None;
            self.is_input_focused = false;
            Ok(NewProfileCreatorAction::Refresh)
        } else {
            self.handle_back_navigation()
        }
    }

    fn move_additional_setting_up(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        self.is_input_focused = false;
        match self.selection_state {
            SelectionState::AdditionalSetting(index) if index > 0 => {
                self.selection_state =
                    SelectionState::AdditionalSetting(index - 1);
            }
            SelectionState::AdditionalSetting(0) => {
                self.selection_state = SelectionState::NextButton;
            }
            SelectionState::NextButton => {
                if !self.additional_settings.is_empty() {
                    self.selection_state = SelectionState::AdditionalSetting(
                        self.additional_settings.len() - 1,
                    );
                }
            }
            _ => {}
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn move_additional_setting_down(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        self.is_input_focused = false;
        match self.selection_state {
            SelectionState::AdditionalSetting(index) => {
                if index < self.additional_settings.len() - 1 {
                    self.selection_state =
                        SelectionState::AdditionalSetting(index + 1);
                } else {
                    self.selection_state = SelectionState::NextButton;
                }
            }
            SelectionState::NextButton => {
                if !self.additional_settings.is_empty() {
                    self.selection_state = SelectionState::AdditionalSetting(0);
                }
            }
            _ => {}
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    async fn handle_confirm_create(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
            KeyCode::Char('q') => self.handle_back_navigation(),
            KeyCode::Enter => {
                if self.ready_to_create {
                    self.move_to_next_step(
                        NewProfileCreationStep::CreatingProfile,
                    )?;
                    self.create_new_profile(0).await
                } else {
                    Ok(NewProfileCreatorAction::WaitForKeyEvent)
                }
            }
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    pub async fn create_new_profile(
        &mut self,
        profile_count: usize,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if !self.ready_to_create {
            return Err(ApplicationError::NotReady(
                "Profile is not ready to be created.".to_string(),
            ));
        }

        let selected_type_index = self.selected_type_index;
        let profile_type = self
            .predefined_types
            .get(selected_type_index)
            .ok_or_else(|| {
                ApplicationError::NotReady(
                    "Invalid provider selected.".to_string(),
                )
            })?;

        let mut settings = Map::new();

        // Add __TEMPLATE.__MODEL_SERVER
        settings.insert(
            "__TEMPLATE.__MODEL_SERVER".to_string(),
            json!(profile_type),
        );

        // Add selected model
        if let Some((_, selected_model)) = self.get_selected_model() {
            settings.insert(
                "__TEMPLATE.MODEL_IDENTIFIER".to_string(),
                json!(selected_model),
            );
        }

        // Add additional settings
        for setting in &self.additional_settings {
            let value = if setting.is_secure {
                json!({
                    "content": setting.value,
                    "encryption_key": "",  // signal that the value must be encrypted
                    "type_info": "string",
                })
            } else if setting.value.is_empty() {
                JsonValue::Null
            } else {
                serde_json::Value::String(setting.value.clone())
            };
            settings.insert(setting.name.clone(), value);
        }

        // Generate a unique profile name if none is provided
        let new_profile_name = if self.new_profile_name.is_empty() {
            format!("New_Profile_{}", profile_count + 1)
        } else {
            self.new_profile_name.clone()
        };

        // Create the profile in the database
        let (tx, rx) = mpsc::channel(1);
        let new_profile_name_clone = new_profile_name.clone();
        let settings_clone = settings.clone();
        let mut db_handler = self.db_handler.clone();

        tokio::spawn(async move {
            let result = db_handler
                .create(&new_profile_name_clone, &json!(settings_clone))
                .await;
            let _ = tx.send(BackgroundTaskResult::ProfileCreated(result)).await;
        });

        self.background_task = Some(rx);
        self.task_start_time = Some(Instant::now());
        self.creation_step = NewProfileCreationStep::CreatingProfile;

        // Reset the state
        self.ready_to_create = false;
        self.skipped_type_selection = false;
        self.selection_state = SelectionState::ProfileType(0);

        Ok(NewProfileCreatorAction::Refresh)
    }

    fn render_creating_profile(&self, f: &mut Frame, area: Rect) {
        if self.background_task.is_some() {
            const SPINNER: &[char] =
                &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

            let elapsed = self
                .task_start_time
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0);

            let spinner_char = SPINNER[(elapsed as usize) % SPINNER.len()];

            let content = format!(
                "{} Creating profile '{}' ... ({} seconds)",
                spinner_char, self.new_profile_name, elapsed
            );

            let paragraph = Paragraph::new(content)
                .style(Style::default().fg(Self::COLOR_HIGHLIGHT))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Creating Profile")
                        .border_style(
                            Style::default().fg(Self::COLOR_SECONDARY),
                        ),
                );

            f.render_widget(paragraph, area);
        }
    }

    fn render_model_selection(&self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        if let Some(profile_type) =
            self.predefined_types.get(self.get_selected_type_index())
        {
            items.push(ListItem::new(Line::from(vec![
                Span::raw("Selected Type: "),
                Span::styled(
                    profile_type,
                    Style::default().fg(Self::COLOR_HIGHLIGHT),
                ),
            ])));
            items.push(ListItem::new(""));

            if let Some(sub_selections) = self.sub_selections.get(profile_type)
            {
                if let Some(model_selection) = sub_selections.first() {
                    items.push(ListItem::new(Span::styled(
                        "Available Models:",
                        Style::default().fg(Self::COLOR_SECONDARY),
                    )));

                    for (index, option) in
                        model_selection.options.iter().enumerate()
                    {
                        let style = if matches!(self.selection_state, SelectionState::ModelOption(selected) if selected == index)
                        {
                            Style::default()
                                .fg(Self::COLOR_HIGHLIGHT)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Self::COLOR_FOREGROUND)
                        };

                        items.push(ListItem::new(Line::from(vec![
                            Span::styled(
                                if matches!(self.selection_state, SelectionState::ModelOption(selected) if selected == index) { "> " } else { "  " },
                                Style::default().fg(Self::COLOR_SECONDARY),
                            ),
                            Span::styled(option, style),
                        ])));
                    }
                }
            }
        }

        let list = List::new(items).block(
            Block::default().borders(Borders::ALL).title("Select Model"),
        );

        f.render_widget(list, area);
    }

    fn render_confirmation(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        let mut items = Vec::new();

        // Provider
        if let Some(provider) =
            self.predefined_types.get(self.selected_type_index)
        {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    "Provider: ",
                    Style::default().fg(Self::COLOR_SECONDARY),
                ),
                Span::styled(
                    provider,
                    Style::default().fg(Self::COLOR_HIGHLIGHT),
                ),
            ])));
        }

        // Selected Model
        if let Some((_, selected_model)) = self.get_selected_model() {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    "Selected Model: ",
                    Style::default().fg(Self::COLOR_SECONDARY),
                ),
                Span::styled(
                    selected_model,
                    Style::default().fg(Self::COLOR_HIGHLIGHT),
                ),
            ])));
        }

        // Additional Settings
        if !self.additional_settings.is_empty() {
            items.push(ListItem::new(""));
            items.push(ListItem::new(Span::styled(
                "Additional Settings:",
                Style::default().fg(Self::COLOR_SECONDARY),
            )));

            for setting in &self.additional_settings {
                let value_display = if setting.is_secure {
                    "*".repeat(setting.value.len())
                } else {
                    setting.value.clone()
                };

                let (display_value, value_style) = if value_display.is_empty() {
                    (
                        "Not set".to_string(),
                        Style::default().fg(Self::COLOR_SECONDARY),
                    )
                } else {
                    (value_display, Style::default().fg(Self::COLOR_HIGHLIGHT))
                };

                let status = if display_value == "Not set" {
                    " (Optional)"
                } else {
                    ""
                };

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{}: ", setting.display_name),
                        Style::default().fg(Self::COLOR_FOREGROUND),
                    ),
                    Span::styled(display_value, value_style),
                    Span::styled(
                        status,
                        Style::default().fg(Self::COLOR_SECONDARY),
                    ),
                ])));
            }
        }

        items.push(ListItem::new(""));
        let ready_message = SimpleString::from(
            "Profile is ready to be created. Press Enter to create the \
             profile.",
        );
        let wrapped_spans = ready_message.wrapped_spans(
            area.width as usize - 4,
            Some(Style::default().fg(Self::COLOR_SUCCESS)),
        );
        for spans in wrapped_spans {
            items.push(ListItem::new(Line::from(spans)));
        }
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm Profile Creation"),
        );

        f.render_widget(list, chunks[0]);

        // Render the Create button
        self.render_next_or_create_button(f, chunks[1]);
    }

    fn confirm_model_selection(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.selection_state {
            SelectionState::ModelOption(index) => {
                self.update_selected_model(index);
                self.move_to_next_button()
            }
            SelectionState::NextButton => {
                // Check if a model is selected
                if self.get_selected_model().is_some() {
                    if self.additional_settings.is_empty() {
                        self.ready_to_create = true;
                        self.move_to_next_step(
                            NewProfileCreationStep::ConfirmCreate,
                        )
                    } else {
                        self.move_to_next_step(
                            NewProfileCreationStep::InputAdditionalSettings,
                        )
                    }
                } else {
                    // If no model is selected, return an error or show a message
                    Ok(NewProfileCreatorAction::Refresh) // You might want to show an error message here
                }
            }
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }
}
