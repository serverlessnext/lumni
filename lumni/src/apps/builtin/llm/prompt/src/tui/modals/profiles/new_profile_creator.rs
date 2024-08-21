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

#[derive(Debug, Clone)]
pub enum SelectionState {
    ProfileType(usize),
    SubOption(usize),
    CreateButton,
}

#[derive(Debug, Clone)]
pub struct SubSelection {
    name: String,
    display_name: String,
    options: Vec<String>,
    selected: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct AdditionalSetting {
    name: String,
    display_name: String,
    value: String,
    is_secure: bool,
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
}

impl NewProfileCreator {
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
                    name: "__TEMPLATE.MODEL_IDENTIFIER".to_string(),
                    display_name: "Model".to_string(),
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
        match key_code {
            KeyCode::Esc | KeyCode::Char('q') => self.handle_back_navigation(),
            _ => self.handle_step_input(key_code).await,
        }
    }

    fn handle_back_navigation(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if self.navigation_stack.len() > 1 {
            self.navigation_stack.pop_back();
            self.creation_step = self
                .navigation_stack
                .back()
                .cloned()
                .unwrap_or(NewProfileCreationStep::EnterName);
            self.reset_step_state();
            Ok(NewProfileCreatorAction::Refresh)
        } else {
            Ok(NewProfileCreatorAction::Cancel)
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
                if let Some(profile_type) =
                    self.predefined_types.get(self.selected_type_index)
                {
                    if let Some(sub_selections) =
                        self.sub_selections.get(profile_type)
                    {
                        if let Some(sub_selection) = sub_selections.first() {
                            if let Some(selected) = sub_selection.selected {
                                self.selection_state =
                                    SelectionState::SubOption(selected);
                            } else {
                                self.selection_state =
                                    SelectionState::SubOption(0);
                            }
                        }
                    }
                }
                self.ready_to_create = false;
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                self.current_additional_setting = 0;
                self.ready_to_create = false;
            }
            NewProfileCreationStep::ConfirmCreate => {
                // Do not reset ready_to_create here
            }
            NewProfileCreationStep::CreatingProfile => {
                // This state shouldn't be reached through back navigation
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
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    async fn handle_step_input(
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

    async fn handle_select_profile_type(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
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

    fn get_max_sub_option_index(&self) -> usize {
        let selected_type_index = self.get_selected_type_index();
        self.predefined_types
            .get(selected_type_index)
            .and_then(|profile_type| self.sub_selections.get(profile_type))
            .and_then(|sub_selections| sub_selections.first())
            .map(|sub_selection| sub_selection.options.len())
            .unwrap_or(0)
    }

    fn update_selected_model(&mut self) {
        if let SelectionState::SubOption(index) = self.selection_state {
            let selected_type_index = self.get_selected_type_index();
            if let Some(profile_type) =
                self.predefined_types.get(selected_type_index)
            {
                if let Some(sub_selections) =
                    self.sub_selections.get_mut(profile_type)
                {
                    if let Some(sub_selection) = sub_selections.first_mut() {
                        sub_selection.selected = Some(index);
                    }
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

    fn render_create_button(&self, f: &mut Frame, area: Rect) {
        let (border_style, text_style) = if self.ready_to_create
            && self.creation_step == NewProfileCreationStep::ConfirmCreate
        {
            (
                Style::default().fg(Self::COLOR_SUCCESS),
                Style::default()
                    .fg(Self::COLOR_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(Color::DarkGray),
                Style::default().fg(Color::DarkGray),
            )
        };

        let button_content = Span::styled("Create Profile", text_style);

        let button = Paragraph::new(button_content)
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            );

        f.render_widget(button, area);
    }

    fn move_to_next_additional_setting(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if self.current_additional_setting < self.additional_settings.len() - 1
        {
            self.current_additional_setting += 1;
            Ok(NewProfileCreatorAction::Refresh)
        } else {
            self.ready_to_create = true;
            self.move_to_next_step(NewProfileCreationStep::ConfirmCreate)
        }
    }

    fn move_to_previous_additional_setting(
        &mut self,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        if self.current_additional_setting > 0 {
            self.current_additional_setting -= 1;
            Ok(NewProfileCreatorAction::Refresh)
        } else {
            // If at the first additional setting, go back to the previous step
            self.handle_back_navigation()
        }
    }

    pub fn get_step_title(&self) -> &'static str {
        match self.creation_step {
            NewProfileCreationStep::EnterName => "Enter Profile Name",
            NewProfileCreationStep::SelectProfileType => "Select Profile Type",
            NewProfileCreationStep::SelectModel => "Select Model",
            NewProfileCreationStep::InputAdditionalSettings => {
                "Additional Settings"
            }
            NewProfileCreationStep::ConfirmCreate => "Confirm Creation",
            NewProfileCreationStep::CreatingProfile => "Creating Profile",
        }
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
                "↑↓: Select Model | Enter: Confirm | Esc: Back to Profile Types"
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
                        self.additional_settings.push(AdditionalSetting {
                            name: format!("__TEMPLATE.{}", key),
                            display_name,
                            value: String::new(),
                            is_secure,
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

                    // Set the selection state to SubOption(0)
                    self.selection_state = SelectionState::SubOption(0);
                    self.ready_to_create = false;

                    self.move_to_next_step(NewProfileCreationStep::SelectModel)
                }
                Ok(_) => Err(ApplicationError::NotReady(
                    "No models available for this server. Please try another \
                     profile type."
                        .to_string(),
                )),
                Err(ApplicationError::NotReady(msg)) => {
                    Err(ApplicationError::NotReady(msg))
                }
                Err(e) => Err(e),
            }
        } else {
            Err(ApplicationError::NotReady(
                "Invalid profile type selected.".to_string(),
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
                self.render_additional_settings_input(f, chunks[1]);
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
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3), // Height for the Create button
            ])
            .split(area);

        let mut items = Vec::new();

        match self.creation_step {
            NewProfileCreationStep::SelectProfileType => {
                if self.skipped_type_selection {
                    items.push(ListItem::new(Line::from(vec![Span::styled(
                        "Profile type selection skipped",
                        Style::default()
                            .fg(Self::COLOR_SECONDARY)
                            .add_modifier(Modifier::ITALIC),
                    )])));
                    items.push(ListItem::new(""));
                    items.push(ListItem::new(Line::from(vec![Span::styled(
                        "Press 'S' to undo skip and select a profile type",
                        Style::default().fg(Self::COLOR_HIGHLIGHT),
                    )])));
                } else {
                    for (i, profile_type) in
                        self.predefined_types.iter().enumerate()
                    {
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
                    items.push(ListItem::new(Line::from(vec![Span::styled(
                        "Press 'S' to skip profile type selection",
                        Style::default().fg(Self::COLOR_SECONDARY),
                    )])));
                }
            }
            NewProfileCreationStep::SelectModel => {
                let selected_type_index = self.get_selected_type_index();
                if let Some(profile_type) =
                    self.predefined_types.get(selected_type_index)
                {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("Selected Type: "),
                        Span::styled(
                            profile_type,
                            Style::default().fg(Self::COLOR_HIGHLIGHT),
                        ),
                    ])));
                    items.push(ListItem::new(""));

                    if let Some(sub_selections) =
                        self.sub_selections.get(profile_type)
                    {
                        for sub_selection in sub_selections {
                            items.push(ListItem::new(Line::from(vec![
                                Span::styled(
                                    &sub_selection.name,
                                    Style::default().fg(Self::COLOR_SECONDARY),
                                ),
                                Span::raw(":"),
                            ])));

                            for (i, option) in
                                sub_selection.options.iter().enumerate()
                            {
                                let is_selected =
                                    sub_selection.selected == Some(i);
                                let is_highlighted = matches!(self.selection_state, SelectionState::SubOption(selected) if selected == i);

                                let style = if is_selected || is_highlighted {
                                    Style::default()
                                        .fg(Self::COLOR_HIGHLIGHT)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Self::COLOR_FOREGROUND)
                                };

                                items.push(ListItem::new(Line::from(vec![
                                    Span::raw("  "),
                                    Span::styled(
                                        if is_selected || is_highlighted {
                                            ">"
                                        } else {
                                            " "
                                        },
                                        Style::default()
                                            .fg(Self::COLOR_SECONDARY),
                                    ),
                                    Span::raw(" "),
                                    Span::styled(option, style),
                                ])));
                            }
                        }
                    }
                }
            }
            NewProfileCreationStep::InputAdditionalSettings => {
                // Render additional settings input
                for (index, setting) in
                    self.additional_settings.iter().enumerate()
                {
                    let style = if index == self.current_additional_setting {
                        Style::default().fg(Self::COLOR_HIGHLIGHT)
                    } else {
                        Style::default().fg(Self::COLOR_FOREGROUND)
                    };

                    let value_display = if setting.is_secure {
                        "*".repeat(setting.value.len())
                    } else {
                        setting.value.clone()
                    };

                    items.push(ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{}: ", setting.display_name),
                            style,
                        ),
                        Span::styled(value_display, style),
                    ])));
                }
            }
            NewProfileCreationStep::ConfirmCreate => {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        "Selected Profile Type: ",
                        Style::default().fg(Self::COLOR_SECONDARY),
                    ),
                    Span::styled(
                        self.predefined_types[self.selected_type_index].clone(),
                        Style::default().fg(Self::COLOR_HIGHLIGHT),
                    ),
                ])));

                if let Some(sub_selections) = self
                    .sub_selections
                    .get(&self.predefined_types[self.selected_type_index])
                {
                    for sub_selection in sub_selections {
                        if let Some(selected) = sub_selection.selected {
                            items.push(ListItem::new(Line::from(vec![
                                Span::styled(
                                    format!(
                                        "Selected {}: ",
                                        sub_selection.name
                                    ),
                                    Style::default().fg(Self::COLOR_SECONDARY),
                                ),
                                Span::styled(
                                    sub_selection.options[selected].clone(),
                                    Style::default().fg(Self::COLOR_HIGHLIGHT),
                                ),
                            ])));
                        }
                    }
                }

                items.push(ListItem::new(""));
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    "Press Enter to create the profile",
                    Style::default().fg(Self::COLOR_SUCCESS),
                )])));
            }
            _ => {}
        }

        let title = self.get_step_title();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(Self::COLOR_HIGHLIGHT)),
            )
            .style(Style::default().bg(Self::COLOR_BACKGROUND));

        f.render_widget(list, chunks[0]);

        // Render the Create button if ready
        if self.ready_to_create {
            self.render_create_button(f, chunks[1]);
        } else {
            // Render an empty block where the button would be
            let empty_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Self::COLOR_FOREGROUND));
            f.render_widget(empty_block, chunks[1]);
        }
    }

    fn get_selected_type_index(&self) -> usize {
        match self.selection_state {
            SelectionState::ProfileType(index) => index,
            SelectionState::SubOption(_) => self.selected_type_index,
            SelectionState::CreateButton => self.selected_type_index,
        }
    }

    fn handle_select_model(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
            KeyCode::Up | KeyCode::Down => {
                let max_index = self.get_max_sub_option_index();
                if max_index > 0 {
                    if let SelectionState::SubOption(index) =
                        self.selection_state
                    {
                        let new_index = if key_code == KeyCode::Up {
                            (index + max_index - 1) % max_index
                        } else {
                            (index + 1) % max_index
                        };
                        self.selection_state =
                            SelectionState::SubOption(new_index);
                        self.update_selected_model();
                    }
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Enter => {
                self.update_selected_model();
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
            }
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    fn handle_input_additional_settings(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
            KeyCode::Enter => self.move_to_next_additional_setting(),
            KeyCode::Backspace => {
                if let Some(setting) = self
                    .additional_settings
                    .get_mut(self.current_additional_setting)
                {
                    setting.value.pop();
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Char(c) => {
                if let Some(setting) = self
                    .additional_settings
                    .get_mut(self.current_additional_setting)
                {
                    setting.value.push(c);
                }
                Ok(NewProfileCreatorAction::Refresh)
            }
            KeyCode::Esc | KeyCode::Up => {
                self.move_to_previous_additional_setting()
            }
            KeyCode::Down | KeyCode::Tab => {
                self.move_to_next_additional_setting()
            }
            _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
        }
    }

    async fn handle_confirm_create(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match key_code {
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

        let selected_type_index = self.get_selected_type_index();
        let profile_type = self
            .predefined_types
            .get(selected_type_index)
            .ok_or_else(|| {
                ApplicationError::NotReady(
                    "Invalid profile type selected.".to_string(),
                )
            })?;

        let mut settings = Map::new();

        // Add __TEMPLATE.__MODEL_SERVER
        settings.insert(
            "__TEMPLATE.__MODEL_SERVER".to_string(),
            json!(profile_type),
        );

        // Add selected model
        if let Some(sub_selections) = self.sub_selections.get(profile_type) {
            if let Some(model_selection) = sub_selections.first() {
                if let Some(selected_index) = model_selection.selected {
                    if let Some(selected_model) =
                        model_selection.options.get(selected_index)
                    {
                        settings.insert(
                            "__TEMPLATE.MODEL_IDENTIFIER".to_string(),
                            json!(selected_model),
                        );
                    }
                }
            }
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

    fn render_additional_settings_input(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3), // For instructions
            ])
            .split(area);

        let mut items = Vec::new();
        for (index, setting) in self.additional_settings.iter().enumerate() {
            let style = if index == self.current_additional_setting {
                Style::default()
                    .fg(Self::COLOR_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Self::COLOR_FOREGROUND)
            };

            let value_display = if setting.is_secure {
                "*".repeat(setting.value.len())
            } else {
                setting.value.clone()
            };

            let status = if value_display.is_empty() {
                " (Optional)"
            } else {
                ""
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{}: ", setting.display_name), style),
                Span::styled(value_display, style),
                Span::styled(
                    status,
                    Style::default().fg(Self::COLOR_SECONDARY),
                ),
            ])));
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Additional Settings"),
        );

        f.render_widget(list, chunks[0]);

        // Render instructions
        let instructions = vec![
            Span::raw("Enter: Save/Skip | "),
            Span::raw("Up/Esc: Previous | "),
            Span::raw("Down/Tab: Next | "),
            Span::raw("Empty value = Skipped"),
        ];
        let instructions_paragraph = Paragraph::new(Line::from(instructions))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Self::COLOR_SECONDARY));

        f.render_widget(instructions_paragraph, chunks[1]);
    }
}
