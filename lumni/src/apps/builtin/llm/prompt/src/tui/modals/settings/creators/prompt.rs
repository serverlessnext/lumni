use ratatui::layout::Margin;
use serde_json::{json, Value as JsonValue};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PromptCreationStep {
    EnterName,
    EnterPrompt,
    ConfirmCreate,
    CreatingPrompt,
}

#[derive(Debug, Clone)]
pub struct PromptCreator {
    name: String,
    prompt: String,
    db_handler: UserProfileDbHandler,
    current_step: PromptCreationStep,
    text_area: Option<TextArea<ReadWriteDocument>>,
}

impl PromptCreator {
    pub fn new(db_handler: UserProfileDbHandler) -> Self {
        Self {
            name: String::new(),
            prompt: String::new(),
            db_handler,
            current_step: PromptCreationStep::EnterName,
            text_area: None,
        }
    }

    pub fn render_creator(&mut self, f: &mut Frame, area: Rect) {
        match self.current_step {
            PromptCreationStep::EnterName => self.render_enter_name(f, area),
            PromptCreationStep::EnterPrompt => {
                self.render_enter_prompt(f, area)
            }
            PromptCreationStep::ConfirmCreate => {
                self.render_confirm_create(f, area)
            }
            PromptCreationStep::CreatingPrompt => {
                self.render_creating_prompt(f, area)
            }
        }
    }

    pub async fn handle_key_event(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match self.current_step {
            PromptCreationStep::EnterName => self.handle_enter_name(input),
            PromptCreationStep::EnterPrompt => self.handle_enter_prompt(input),
            PromptCreationStep::ConfirmCreate => {
                self.handle_confirm_create(input).await
            }
            PromptCreationStep::CreatingPrompt => Ok(CreatorAction::Continue),
        }
    }

    fn render_enter_name(&self, f: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.name.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Enter Prompt Name"),
            );
        f.render_widget(input, area);
    }

    fn render_enter_prompt(&mut self, f: &mut Frame, area: Rect) {
        if self.text_area.is_none() {
            // Initialize with dummy text
            let dummy_text =
                vec![TextLine::from_text("Enter your prompt here...", None)];
            self.text_area =
                Some(TextArea::with_read_write_document(Some(dummy_text)));
        }

        if let Some(text_area) = &mut self.text_area {
            let block =
                Block::default().borders(Borders::ALL).title("Enter Prompt");
            let inner_area = block.inner(area);
            f.render_widget(block, area);
            text_area.render(f, inner_area);
        }
    }

    fn render_confirm_create(&mut self, f: &mut Frame, area: Rect) {
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
            .title("Prompt Details");

        if self.text_area.is_none() {
            self.text_area = Some(TextArea::with_read_write_document(Some(
                self.create_confirm_details(),
            )));
        }

        if let Some(text_area) = &mut self.text_area {
            text_area.render(f, content_area[0].inner(Margin::new(1, 1)));
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

        let create_button = Paragraph::new("[ Create Prompt ]")
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(create_button, button_chunks[1]);
    }

    fn render_creating_prompt(&self, f: &mut Frame, area: Rect) {
        let content = format!("Creating prompt '{}'...", self.name);

        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Creating Prompt"),
            );

        f.render_widget(paragraph, area);
    }

    fn handle_enter_name(
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
                    self.current_step = PromptCreationStep::EnterPrompt;
                }
                Ok(CreatorAction::Continue)
            }
            KeyCode::Backspace => {
                self.name.pop();
                Ok(CreatorAction::Continue)
            }
            KeyCode::Esc => Ok(CreatorAction::Cancel),
            _ => Ok(CreatorAction::Continue),
        }
    }

    fn handle_enter_prompt(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        if let Some(text_area) = &mut self.text_area {
            match input.code {
                KeyCode::Enter => {
                    // For now, we'll just use a dummy prompt
                    self.prompt = "Dummy prompt text".to_string();
                    self.current_step = PromptCreationStep::ConfirmCreate;
                    self.text_area = None;
                    Ok(CreatorAction::Continue)
                }
                KeyCode::Esc => {
                    self.current_step = PromptCreationStep::EnterName;
                    self.text_area = None;
                    Ok(CreatorAction::Continue)
                }
                _ => {
                    text_area.handle_key_event(input);
                    Ok(CreatorAction::Continue)
                }
            }
        } else {
            Ok(CreatorAction::Continue)
        }
    }

    async fn handle_confirm_create(
        &mut self,
        input: KeyEvent,
    ) -> Result<CreatorAction<ConfigItem>, ApplicationError> {
        match input.code {
            KeyCode::Enter => {
                self.current_step = PromptCreationStep::CreatingPrompt;
                match self.create_prompt().await {
                    Ok(new_config) => Ok(CreatorAction::Finish(new_config)),
                    Err(e) => {
                        log::error!("Failed to create prompt: {}", e);
                        self.current_step = PromptCreationStep::ConfirmCreate;
                        Ok(CreatorAction::Continue)
                    }
                }
            }
            KeyCode::Esc | KeyCode::Backspace => {
                self.current_step = PromptCreationStep::EnterPrompt;
                self.text_area = None;
                Ok(CreatorAction::Continue)
            }
            _ => Ok(CreatorAction::Continue),
        }
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

        let mut prompt_line = TextLine::new();
        prompt_line.add_segment(
            "Prompt:",
            Some(Style::default().add_modifier(Modifier::BOLD)),
        );
        lines.push(prompt_line);

        for line in self.prompt.lines() {
            let mut prompt_value_line = TextLine::new();
            prompt_value_line.add_segment(
                format!("  {}", line),
                Some(Style::default().fg(Color::Cyan)),
            );
            lines.push(prompt_value_line);
        }

        lines
    }

    pub async fn create_prompt(
        &mut self,
    ) -> Result<ConfigItem, ApplicationError> {
        let new_config = self
            .db_handler
            .create_configuration_item(
                self.name.clone(),
                "prompt",
                json!({ "content": self.prompt }),
            )
            .await?;

        Ok(ConfigItem::DatabaseConfig(new_config))
    }
}
