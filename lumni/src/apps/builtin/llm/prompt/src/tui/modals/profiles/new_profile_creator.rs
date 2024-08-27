use std::collections::{HashMap, VecDeque};

use super::*;

// TODO notes:
// - add ability to attach filepaths to a profile. This should be done as a NewProfileCreationStep. As a first step, just make it a simple input field.
// - replace input field in previous step by an EditWindow that can contain multiple lines for input

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
    const COLOR_PLACEHOLDER: Color = Color::DarkGray;

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
        }
    }

    fn handle_back_navigation(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.creation_step {
            NewProfileCreationStep::SelectProfileType => {
                if !self.new_profile_name.is_empty() {
                    self.creation_step = NewProfileCreationStep::EnterName;
                    self.navigation_stack.pop_back();
                    Ok(NewProfileCreatorAction::Refresh)
                } else {
                    Ok(NewProfileCreatorAction::Cancel)
                }
            }
            NewProfileCreationStep::SelectModel => {
                self.creation_step = NewProfileCreationStep::SelectProfileType;
                self.navigation_stack.pop_back();
                self.selection_state =
                    SelectionState::ProfileType(self.selected_type_index);
                self.skipped_type_selection = false; // Reset skipped state
                Ok(NewProfileCreatorAction::Refresh)
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                self.creation_step = NewProfileCreationStep::SelectModel;
                self.navigation_stack.pop_back();
                if let Some((index, _)) = self.get_selected_model() {
                    self.selection_state = SelectionState::ModelOption(index);
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            NewProfileCreationStep::ConfirmCreate => {
                self.ready_to_create = false;
                if self.skipped_type_selection {
                    // Always go back to provider selection when skipped
                    self.creation_step =
                        NewProfileCreationStep::SelectProfileType;
                    self.skipped_type_selection = false;
                    self.selection_state =
                        SelectionState::ProfileType(self.selected_type_index);
                } else if self.additional_settings.is_empty() {
                    self.creation_step = NewProfileCreationStep::SelectModel;
                    if let Some((index, _)) = self.get_selected_model() {
                        self.selection_state =
                            SelectionState::ModelOption(index);
                    }
                } else {
                    self.creation_step =
                        NewProfileCreationStep::InputAdditionalSettings;
                    self.selection_state = SelectionState::AdditionalSetting(
                        self.additional_settings.len() - 1,
                    );
                }
                self.navigation_stack.pop_back();
                Ok(NewProfileCreatorAction::Refresh)
            }
            _ => Ok(NewProfileCreatorAction::Cancel),
        }
    }

    fn reset_step_state(&mut self) {
        match self.creation_step {
            NewProfileCreationStep::EnterName => {
                self.ready_to_create = false;
                self.skipped_type_selection = false;
            }
            NewProfileCreationStep::SelectProfileType => {
                self.ready_to_create = false;
                // Don't reset skipped_type_selection here
                self.selection_state =
                    SelectionState::ProfileType(self.selected_type_index);
            }
            NewProfileCreationStep::SelectModel => {
                self.ready_to_create = false;
                self.skipped_type_selection = false;
                if let Some((index, _)) = self.get_selected_model() {
                    self.selection_state = SelectionState::ModelOption(index);
                }
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                self.ready_to_create = false;
                self.skipped_type_selection = false;
                if !self.additional_settings.is_empty() {
                    self.selection_state = SelectionState::AdditionalSetting(0);
                } else {
                    // Skip this step if there are no additional settings
                    _ = self.move_to_next_step(
                        NewProfileCreationStep::ConfirmCreate,
                    );
                }
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
            KeyCode::Enter | KeyCode::Tab => {
                if !self.new_profile_name.is_empty() {
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
            KeyCode::Up => {
                if let SelectionState::ProfileType(index) =
                    &mut self.selection_state
                {
                    if *index > 0 {
                        *index -= 1;
                        self.selected_type_index = *index;
                    }
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Down => {
                if let SelectionState::ProfileType(index) =
                    &mut self.selection_state
                {
                    if *index < self.predefined_types.len() - 1 {
                        *index += 1;
                        self.selected_type_index = *index;
                    }
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Enter | KeyCode::Tab => {
                self.selected_type_index = self.get_selected_type_index();
                self.prepare_for_model_selection().await
            }
            KeyCode::Esc => self.handle_back_navigation(),
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

    pub fn get_instructions(&self, width: u16) -> Vec<Vec<Span<'static>>> {
        let instructions = match self.creation_step {
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
        };

        let simple_string = SimpleString::from(instructions);
        simple_string.wrapped_spans(
            width as usize,
            Some(Style::default().fg(Self::COLOR_SECONDARY)),
            Some(" | "),
        )
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

    async fn prepare_for_model_selection(
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

                    // Update or create the SubSelection for models
                    self.sub_selections
                        .entry(profile_type.clone())
                        .or_insert_with(|| {
                            vec![SubSelection {
                                options: Vec::new(),
                                selected: None,
                            }]
                        })
                        .first_mut()
                        .map(|sub_selection| {
                            sub_selection.options = model_options;
                            sub_selection.selected = Some(0);
                        });

                    // Prepare additional settings
                    self.prepare_additional_settings(&model_server);

                    // Set the selection state to ModelOption(0)
                    self.selection_state = SelectionState::ModelOption(0);
                    self.ready_to_create = false;
                    self.skipped_type_selection = false;

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

    pub fn render_main_content(&self, f: &mut Frame, area: Rect) {
        // Render the main content (everything except the button)
        match self.creation_step {
            NewProfileCreationStep::EnterName => {
                self.render_profile_name_input(f, area);
            }
            NewProfileCreationStep::SelectProfileType => {
                self.render_profile_type_selection(f, area);
            }
            NewProfileCreationStep::SelectModel => {
                self.render_model_selection(f, area);
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                self.render_additional_settings(f, area);
            }
            NewProfileCreationStep::ConfirmCreate => {
                self.render_confirmation(f, area);
            }
            NewProfileCreationStep::CreatingProfile => {
                self.render_creating_profile(f, area);
            }
        }
    }

    fn render_profile_name_input(&self, f: &mut Frame, area: Rect) {
        let name_color = Self::COLOR_HIGHLIGHT;
        let border_color =
            if self.creation_step == NewProfileCreationStep::EnterName {
                Self::COLOR_HIGHLIGHT
            } else {
                Self::COLOR_FOREGROUND
            };

        let input = Paragraph::new(Line::from(vec![
            Span::raw("> "),
            Span::styled(
                &self.new_profile_name,
                Style::default().fg(name_color),
            ),
        ]))
        .style(Style::default().fg(Self::COLOR_FOREGROUND))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title("Enter New Profile Name"),
        );
        f.render_widget(input, area);
    }

    fn create_wrapped_spans(
        &self,
        message: String,
        width: usize,
        style: Style,
    ) -> Vec<ListItem> {
        let simple_string = SimpleString::from(message);
        let wrapped_spans =
            simple_string.wrapped_spans(width, Some(style), None);
        wrapped_spans
            .into_iter()
            .map(Line::from)
            .map(ListItem::new)
            .collect()
    }

    fn render_profile_type_selection(&self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        if self.skipped_type_selection {
            items.extend(
                self.create_wrapped_spans(
                    "Provider selection skipped".to_string(),
                    area.width as usize - 4,
                    Style::default()
                        .fg(Self::COLOR_SECONDARY)
                        .add_modifier(Modifier::ITALIC),
                ),
            );

            items.push(ListItem::new(""));

            items.extend(
                self.create_wrapped_spans(
                    "Profile is ready to be created. Press Enter to create \
                     the profile."
                        .to_string(),
                    area.width as usize - 4,
                    Style::default().fg(Self::COLOR_SUCCESS),
                ),
            );
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
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Provider"),
        );

        f.render_widget(list, area);
    }

    fn render_additional_settings(&self, f: &mut Frame, area: Rect) {
        let available_width = area.width as usize - 3; // Subtract 3 for the left border and spacing

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

            let placeholder_style =
                Style::default().fg(Self::COLOR_PLACEHOLDER);

            let arrow = if is_selected { ">" } else { " " };

            // Prepare the key (display name) part
            let key_part = format!("{}: ", setting.display_name);
            let key_width = key_part.chars().count();

            // Prepare the value part
            let (input_content, content_style) = if self.is_input_focused
                && self.selection_state
                    == SelectionState::AdditionalSetting(index)
            {
                (
                    self.temp_input.as_ref().unwrap_or(&setting.value),
                    input_style,
                )
            } else if setting.value.is_empty() {
                (&setting.placeholder, placeholder_style)
            } else {
                (&setting.value, input_style)
            };

            // Render the key-value pair
            let mut first_line = true;
            let mut remaining_content = input_content.to_string();

            while !remaining_content.is_empty() || first_line {
                let mut styled_spans = Vec::new();

                if first_line {
                    styled_spans.push(Span::styled(
                        arrow,
                        Style::default().fg(Self::COLOR_SECONDARY),
                    ));
                    styled_spans.push(Span::raw(" "));
                    styled_spans
                        .push(Span::styled(key_part.clone(), label_style));

                    let available_value_width =
                        available_width.saturating_sub(key_width + 2);
                    let (line, rest) = split_at_width(
                        &remaining_content,
                        available_value_width,
                    );
                    styled_spans.push(Span::styled(line, content_style));
                    remaining_content = rest.to_string();
                } else {
                    styled_spans.push(Span::raw("  ")); // Indent continuation lines
                    let (line, rest) =
                        split_at_width(&remaining_content, available_width - 2);
                    styled_spans.push(Span::styled(line, content_style));
                    remaining_content = rest.to_string();
                }

                items.push(ListItem::new(Line::from(styled_spans)));
                first_line = false;
            }
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Additional Settings"),
        );

        f.render_widget(list, area);
    }

    pub fn render_next_or_create_button(&self, f: &mut Frame, area: Rect) {
        let (button_text, button_style, is_selected) = if self
            .skipped_type_selection
        {
            ("Create", Style::default().fg(Self::COLOR_SUCCESS), true)
        } else {
            match self.creation_step {
                NewProfileCreationStep::ConfirmCreate => {
                    ("Create", Style::default().fg(Self::COLOR_SUCCESS), true)
                }
                _ => (
                    "Next",
                    Style::default().fg(Self::COLOR_HIGHLIGHT),
                    matches!(self.selection_state, SelectionState::NextButton),
                ),
            }
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
            KeyCode::Enter | KeyCode::Tab => self.confirm_model_selection(),
            KeyCode::Esc => self.handle_back_navigation(),
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    fn move_model_selection_up(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::ModelOption(index) = self.selection_state {
            if index > 0 {
                self.selection_state = SelectionState::ModelOption(index - 1);
            }
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn move_model_selection_down(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::ModelOption(index) = self.selection_state {
            let max_index = self.get_max_model_option_index();
            if max_index > 0 && index < max_index - 1 {
                self.selection_state = SelectionState::ModelOption(index + 1);
            }
        }
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
            KeyCode::Esc => self.handle_back_navigation(),
            KeyCode::Tab => {
                self.move_to_next_step(NewProfileCreationStep::ConfirmCreate)
            }
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    fn handle_enter_additional_setting(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::AdditionalSetting(current_index) =
            self.selection_state
        {
            if !self.is_input_focused {
                // Start editing
                self.temp_input =
                    Some(self.additional_settings[current_index].value.clone());
                self.is_input_focused = true;
                Ok(NewProfileCreatorAction::Refresh)
            } else {
                // Finish editing
                if let Some(temp) = self.temp_input.take() {
                    self.additional_settings[current_index].value = temp;
                }
                self.is_input_focused = false;

                if current_index + 1 < self.additional_settings.len() {
                    // Move to the next setting
                    self.selection_state =
                        SelectionState::AdditionalSetting(current_index + 1);
                    Ok(NewProfileCreatorAction::Refresh)
                } else {
                    // Last setting completed, move to confirm create
                    self.ready_to_create = true;
                    self.move_to_next_step(
                        NewProfileCreationStep::ConfirmCreate,
                    )
                }
            }
        } else {
            Ok(NewProfileCreatorAction::WaitForKeyEvent)
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

    fn move_additional_setting_up(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::AdditionalSetting(index) = self.selection_state {
            if index > 0 {
                self.selection_state =
                    SelectionState::AdditionalSetting(index - 1);
            }
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    fn move_additional_setting_down(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::AdditionalSetting(index) = self.selection_state {
            if index < self.additional_settings.len() - 1 {
                self.selection_state =
                    SelectionState::AdditionalSetting(index + 1);
            }
        }
        Ok(NewProfileCreatorAction::Refresh)
    }

    async fn handle_confirm_create(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc => self.handle_back_navigation(),
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
                        let is_selected = matches!(self.selection_state, SelectionState::ModelOption(selected) if selected == index);

                        let style = if is_selected {
                            Style::default()
                                .fg(Self::COLOR_HIGHLIGHT)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Self::COLOR_FOREGROUND)
                        };

                        items.push(ListItem::new(Line::from(vec![
                            Span::styled(
                                if is_selected { "> " } else { "  " },
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
                        "skipped".to_string(),
                        Style::default().fg(Self::COLOR_PLACEHOLDER),
                    )
                } else {
                    (value_display, Style::default().fg(Self::COLOR_HIGHLIGHT))
                };

                // Prepare the key (display name) part
                let key_part = format!("{}: ", setting.display_name);
                let key_width = key_part.chars().count();

                // Wrap the value part using SimpleString
                let value_width = area.width as usize - 4 - key_width;
                let simple_string = SimpleString::from(display_value);
                let wrapped_value = simple_string.wrapped_spans(
                    value_width,
                    Some(value_style),
                    None,
                );

                // Render the key-value pair
                for (i, value_spans) in wrapped_value.into_iter().enumerate() {
                    let mut styled_spans = Vec::new();

                    if i == 0 {
                        styled_spans.push(Span::styled(
                            key_part.clone(),
                            Style::default().fg(Self::COLOR_FOREGROUND),
                        ));
                    } else {
                        styled_spans.push(Span::raw(" ".repeat(key_width)));
                    }

                    styled_spans.extend(value_spans);

                    items.push(ListItem::new(Line::from(styled_spans)));
                }
            }
        }

        items.push(ListItem::new(""));
        items.extend(self.create_wrapped_spans(
            String::from(
                "Profile is ready to be created. Press Enter to create the \
                 profile.",
            ),
            area.width as usize - 4,
            Style::default().fg(Self::COLOR_SUCCESS),
        ));

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm Profile Creation"),
        );

        f.render_widget(list, area);
    }

    fn confirm_model_selection(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if let SelectionState::ModelOption(index) = self.selection_state {
            self.update_selected_model(index);

            if !self.additional_settings.is_empty() {
                self.move_to_next_step(
                    NewProfileCreationStep::InputAdditionalSettings,
                )
            } else {
                self.ready_to_create = true;
                self.move_to_next_step(NewProfileCreationStep::ConfirmCreate)
            }
        } else {
            Ok(NewProfileCreatorAction::WaitForKeyEvent)
        }
    }
}
fn split_at_width(s: &str, width: usize) -> (String, String) {
    let mut chars = s.chars().peekable();
    let mut line = String::new();
    let mut line_width = 0;

    while let Some(c) = chars.next() {
        let char_width = if c == '\t' { 4 } else { 1 }; // Assume tab width of 4
        if line_width + char_width > width {
            return (line, chars.collect());
        }
        line.push(c);
        line_width += char_width;
    }

    (line, String::new())
}
