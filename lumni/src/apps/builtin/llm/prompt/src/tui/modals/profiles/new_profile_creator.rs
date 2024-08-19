use std::collections::HashMap;

use super::*;

#[derive(Debug)]
pub enum BackgroundTaskResult {
    ProfileCreated(Result<String, ApplicationError>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NewProfileCreationStep {
    EnterName,
    SelectProfileType,
    SelectSubOption,
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

#[derive(Debug)]
pub struct SubSelection {
    name: String,
    options: Vec<String>,
    selected: Option<usize>,
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
                    name: "Model".to_string(),
                    options: vec![],
                    selected: None,
                }],
            );
        }

        Self {
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
        }
    }

    pub async fn prepare_for_model_selection(
        &mut self,
    ) -> Result<(), ApplicationError> {
        let selected_type_index = self.get_selected_type_index();
        if let Some(profile_type) =
            self.predefined_types.get(selected_type_index).cloned()
        {
            let model_server = ModelServer::from_str(&profile_type)?;

            match model_server.list_models().await {
                Ok(models) if !models.is_empty() => {
                    let model_options: Vec<String> =
                        models.iter().map(|m| m.identifier.0.clone()).collect();

                    // Create a new SubSelection for models
                    let model_selection = SubSelection {
                        name: "Model".to_string(),
                        options: model_options,
                        selected: Some(0),
                    };

                    // Update or insert the sub_selection for this profile type
                    self.sub_selections
                        .insert(profile_type, vec![model_selection]);

                    self.creation_step =
                        NewProfileCreationStep::SelectSubOption;
                    self.selection_state = SelectionState::SubOption(0);
                    self.ready_to_create = false; // Reset this flag
                    Ok(())
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
                Constraint::Min(1),    // Type and sub-option selection
            ])
            .split(area);

        self.render_profile_name_input(f, chunks[0]);
        self.render_type_and_sub_options(f, chunks[1]);
    }

    fn render_profile_name_input(&self, f: &mut Frame, area: Rect) {
        let name_color =
            if self.creation_step == NewProfileCreationStep::EnterName {
                Self::COLOR_HIGHLIGHT
            } else {
                Self::COLOR_FOREGROUND
            };

        let input = Paragraph::new(Line::from(vec![
            Span::raw("| "),
            Span::styled(
                &self.new_profile_name,
                Style::default().fg(name_color),
            ),
            Span::raw(" |"),
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
            NewProfileCreationStep::CreatingProfile => {
                self.render_creating_profile(f, area);
                return;
            }
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
            NewProfileCreationStep::SelectSubOption
            | NewProfileCreationStep::EnterName => {
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

    pub async fn handle_input(
        &mut self,
        key_code: KeyCode,
    ) -> Result<NewProfileCreatorAction, ApplicationError> {
        match self.creation_step {
            NewProfileCreationStep::EnterName => match key_code {
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
                        self.creation_step =
                            NewProfileCreationStep::SelectProfileType;
                        self.skipped_type_selection = false; // Reset skip flag
                        self.selection_state = SelectionState::ProfileType(0); // Reset selection
                        self.ready_to_create = false; // Reset ready flag
                        Ok(NewProfileCreatorAction::Refresh)
                    } else {
                        Ok(NewProfileCreatorAction::WaitForKeyEvent)
                    }
                }
                KeyCode::Esc => Ok(NewProfileCreatorAction::Cancel),
                _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
            },
            NewProfileCreationStep::SelectProfileType => match key_code {
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
                        self.creation_step =
                            NewProfileCreationStep::CreatingProfile;
                        self.create_new_profile(0).await?; // Assuming 0 as profile_count, adjust as needed
                        Ok(NewProfileCreatorAction::Refresh)
                    } else {
                        self.selected_type_index =
                            self.get_selected_type_index();
                        match self.prepare_for_model_selection().await {
                            Ok(()) => Ok(NewProfileCreatorAction::Refresh),
                            Err(e) => Err(e),
                        }
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.skipped_type_selection = !self.skipped_type_selection;
                    self.ready_to_create = self.skipped_type_selection;
                    if self.skipped_type_selection {
                        self.selection_state = SelectionState::CreateButton;
                    } else {
                        self.selection_state = SelectionState::ProfileType(0);
                    }
                    Ok(NewProfileCreatorAction::Refresh)
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    // Go back to EnterName step
                    self.creation_step = NewProfileCreationStep::EnterName;
                    self.skipped_type_selection = false;
                    self.ready_to_create = false;
                    Ok(NewProfileCreatorAction::Refresh)
                }
                _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
            },
            NewProfileCreationStep::SelectSubOption => match key_code {
                KeyCode::Up => {
                    if let SelectionState::SubOption(index) =
                        self.selection_state
                    {
                        if index > 0 {
                            self.selection_state =
                                SelectionState::SubOption(index - 1);
                        }
                    }
                    self.update_selected_model();
                    Ok(NewProfileCreatorAction::Refresh)
                }
                KeyCode::Down => {
                    let max_index = self.get_max_sub_option_index();
                    if let SelectionState::SubOption(index) =
                        self.selection_state
                    {
                        if index < max_index - 1 {
                            self.selection_state =
                                SelectionState::SubOption(index + 1);
                        }
                    }
                    self.update_selected_model();
                    Ok(NewProfileCreatorAction::Refresh)
                }
                KeyCode::Enter => {
                    self.update_selected_model();
                    self.ready_to_create = true;
                    self.selection_state = SelectionState::CreateButton;
                    Ok(NewProfileCreatorAction::Refresh)
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.creation_step =
                        NewProfileCreationStep::SelectProfileType;
                    self.selection_state = SelectionState::ProfileType(
                        self.get_selected_type_index(),
                    );
                    Ok(NewProfileCreatorAction::Refresh)
                }
                _ => Ok(NewProfileCreatorAction::WaitForKeyEvent),
            },
            NewProfileCreationStep::CreatingProfile => {
                // Once profile creation has started, we don't allow going back
                Ok(NewProfileCreatorAction::WaitForKeyEvent)
            }
        }
    }

    fn get_selected_type_index(&self) -> usize {
        match self.selection_state {
            SelectionState::ProfileType(index) => index,
            SelectionState::SubOption(_) => self.selected_type_index,
            SelectionState::CreateButton => self.selected_type_index,
        }
    }

    pub async fn create_new_profile(
        &mut self,
        profile_count: usize,
    ) -> Result<(), ApplicationError> {
        if !self.ready_to_create {
            return Err(ApplicationError::NotReady(
                "Profile is not ready to be created.".to_string(),
            ));
        }

        let selected_type_index = self.get_selected_type_index();
        let profile_type = if self.skipped_type_selection {
            None
        } else {
            self.predefined_types.get(selected_type_index)
        };

        let mut settings = Map::new();

        if let Some(ptype) = profile_type {
            settings.insert("__PROFILE_TYPE".to_string(), json!(ptype));

            if let Some(sub_selections) = self.sub_selections.get(ptype) {
                for sub_selection in sub_selections {
                    if let Some(selected_index) = sub_selection.selected {
                        if let Some(selected_option) =
                            sub_selection.options.get(selected_index)
                        {
                            settings.insert(
                                sub_selection.name.clone(),
                                json!(selected_option),
                            );
                        }
                    }
                }
            }
        }

        // Generate a unique profile name if none is provided
        let new_profile_name = if self.new_profile_name.is_empty() {
            format!("New_Profile_{}", profile_count + 1)
        } else {
            self.new_profile_name.clone()
        };

        // Create or update the profile in the database
        let (tx, rx) = mpsc::channel(1);
        let new_profile_name_clone = new_profile_name.clone();
        let settings_clone = settings.clone();
        let mut db_handler = self.db_handler.clone();

        tokio::spawn(async move {
            let result = db_handler
                .create_or_update(
                    &new_profile_name_clone,
                    &json!(settings_clone),
                )
                .await;
            let _ = tx
                .send(BackgroundTaskResult::ProfileCreated(
                    result.map(|_| new_profile_name_clone),
                ))
                .await;
        });

        self.background_task = Some(rx);
        self.task_start_time = Some(Instant::now());
        self.creation_step = NewProfileCreationStep::CreatingProfile;

        // Reset the state
        self.ready_to_create = false;
        self.skipped_type_selection = false;
        self.selection_state = SelectionState::ProfileType(0);
        Ok(())
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

    pub fn get_instructions(&self) -> &'static str {
        match self.creation_step {
            NewProfileCreationStep::EnterName => {
                "Enter profile name | Enter: Confirm | Esc: Cancel"
            }
            NewProfileCreationStep::SelectProfileType => {
                "↑↓: Select Type | Enter: Confirm | Esc: Cancel"
            }
            NewProfileCreationStep::SelectSubOption => {
                "↑↓: Select Model | Enter: Confirm | Esc: Back to Profile Types"
            }
            NewProfileCreationStep::CreatingProfile => "Creating profile...",
        }
    }

    fn render_create_button(&self, f: &mut Frame, area: Rect) {
        let (border_style, text_style) = if self.ready_to_create {
            if matches!(self.selection_state, SelectionState::CreateButton) {
                (
                    Style::default().fg(Self::COLOR_SUCCESS),
                    Style::default()
                        .fg(Self::COLOR_SUCCESS)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    Style::default().fg(Self::COLOR_HIGHLIGHT),
                    Style::default().fg(Self::COLOR_HIGHLIGHT),
                )
            }
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

    fn get_step_title(&self) -> &'static str {
        match self.creation_step {
            NewProfileCreationStep::EnterName => "Enter Profile Name",
            NewProfileCreationStep::SelectProfileType => "Select Profile Type",
            NewProfileCreationStep::SelectSubOption => "Select Model",
            NewProfileCreationStep::CreatingProfile => "Creating Profile",
        }
    }
}
