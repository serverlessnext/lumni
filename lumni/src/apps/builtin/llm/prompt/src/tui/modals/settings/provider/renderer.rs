use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use super::manager::ProviderManager;
use super::{EditMode, GenericList, TabFocus};

pub struct ProviderEditRenderer;

impl ProviderEditRenderer {
    pub fn new() -> Self {
        ProviderEditRenderer
    }

    pub fn render_layout(
        &self,
        f: &mut Frame,
        area: Rect,
        provider_manager: &ProviderManager,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(1),    // Main content
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        self.render_title(f, chunks[0]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            .split(chunks[1]);

        self.render_provider_list(f, main_chunks[0], provider_manager);
        self.render_content(f, main_chunks[1], provider_manager);

        self.render_instructions(f, chunks[2], provider_manager);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("Provider Editor")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        f.render_widget(title, area);
    }

    fn render_provider_list(
        &self,
        f: &mut Frame,
        area: Rect,
        provider_manager: &ProviderManager,
    ) {
        let items: Vec<ListItem> = provider_manager
            .list
            .get_items()
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let content = if i == provider_manager.list.get_selected_index()
                    && provider_manager.get_rename_buffer().is_some()
                {
                    format!("{}", provider_manager.get_rename_buffer().unwrap())
                } else {
                    item.clone()
                };

                let style = if i == provider_manager.list.get_selected_index() {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else if i == provider_manager.list.get_items().len() - 1 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                ListItem::new(Span::styled(content, style))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Providers"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(provider_manager.list.get_selected_index()));

        f.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_content(
        &self,
        f: &mut Frame,
        area: Rect,
        provider_manager: &ProviderManager,
    ) {
        match provider_manager.tab_focus {
            TabFocus::Settings | TabFocus::List => {
                self.render_settings(f, area, provider_manager)
            }
            TabFocus::Creation => {
                if let Some(creator) = &provider_manager.creator {
                    creator.render(f, area);
                }
            }
        }
    }

    fn render_settings(
        &self,
        f: &mut Frame,
        area: Rect,
        provider_manager: &ProviderManager,
    ) {
        if let Some(provider) = provider_manager.list.get_selected_provider() {
            let mut items = Vec::new();

            items.push(ListItem::new(format!("Name: {}", provider.name)));
            items.push(ListItem::new(format!(
                "Type: {}",
                provider.provider_type
            )));

            if let Some(model) = &provider.model_identifier {
                items.push(ListItem::new(format!("Model: {}", model)));
            } else {
                items.push(ListItem::new("Model: Not set"));
            }

            items.push(ListItem::new("Additional Settings:"));
            for (key, setting) in &provider.additional_settings {
                let value_display = if setting.is_secure {
                    "*".repeat(setting.value.len())
                } else {
                    setting.value.clone()
                };

                let content =
                    format!("  {}: {}", setting.display_name, value_display);
                let style =
                    if provider_manager.settings_editor.get_current_key()
                        == Some(key.as_str())
                    {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                items.push(ListItem::new(Span::styled(content, style)));
            }

            if let EditMode::EditingValue =
                provider_manager.settings_editor.edit_mode
            {
                if let Some(current_key) =
                    provider_manager.settings_editor.get_current_key()
                {
                    let edit_content = format!(
                        "Editing {}: {}",
                        current_key,
                        provider_manager.settings_editor.get_edit_buffer()
                    );
                    items.push(ListItem::new(Span::styled(
                        edit_content,
                        Style::default().fg(Color::Cyan),
                    )));
                }
            }

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Provider Settings"),
                )
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            f.render_widget(list, area);
        } else {
            let paragraph = Paragraph::new("No provider selected").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Provider Settings"),
            );
            f.render_widget(paragraph, area);
        }
    }

    fn render_instructions(
        &self,
        f: &mut Frame,
        area: Rect,
        provider_manager: &ProviderManager,
    ) {
        let instructions = match (
            provider_manager.tab_focus,
            provider_manager.settings_editor.edit_mode,
        ) {
            (TabFocus::List, _) => {
                "↑↓: Navigate | Enter: Select | R: Rename | D: Delete | Tab: \
                 Switch Tab"
            }
            (TabFocus::Settings, EditMode::NotEditing) => {
                "↑↓: Navigate | Enter: Edit | n: New | N: New Secure | D: \
                 Delete | C: Clear | S: Show/Hide Secure | ←/Tab/q/Esc: Back \
                 to List"
            }
            (TabFocus::Settings, EditMode::EditingValue) => {
                "Enter: Save | Esc: Cancel"
            }
            (TabFocus::Settings, EditMode::AddingNewKey) => {
                "Enter: Confirm Key | Esc: Cancel"
            }
            (TabFocus::Settings, EditMode::AddingNewValue) => {
                "Enter: Save New Value | Esc: Cancel"
            }
            (TabFocus::Creation, _) => "Enter: Create Provider | Esc: Cancel",
        };

        let paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));
        f.render_widget(paragraph, area);
    }
}
