use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph,
    Tabs,
};

use super::{EditMode, EditTab, SettingsModal, SimpleString, TabFocus};

pub struct ProfileEditRenderer;

impl ProfileEditRenderer {
    pub fn new() -> Self {
        ProfileEditRenderer
    }

    pub fn render_layout(
        &self,
        f: &mut Frame,
        area: Rect,
        modal: &SettingsModal,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(1),    // Main content
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        self.render_tab_bar(f, chunks[0], modal.current_tab);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            .split(chunks[1]);

        self.render_list(f, main_chunks[0], modal);
        self.render_content(f, main_chunks[1], modal);

        self.render_instructions(f, chunks[2], modal);
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect, current_tab: EditTab) {
        let tabs = vec!["Profiles", "Providers"];
        let tab_index = match current_tab {
            EditTab::Profiles => 0,
            EditTab::Providers => 1,
        };
        let tabs = Tabs::new(tabs)
            .block(Block::default().borders(Borders::ALL))
            .select(tab_index)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::Yellow));
        f.render_widget(tabs, area);
    }

    fn render_list(&self, f: &mut Frame, area: Rect, modal: &SettingsModal) {
        let items: Vec<ListItem> = modal
            .get_current_list()
            .get_items()
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let content = if i
                    == modal.get_current_list().get_selected_index()
                    && modal.get_rename_buffer().is_some()
                {
                    // Show rename buffer for the selected item if renaming
                    format!("{}", modal.get_rename_buffer().unwrap())
                } else {
                    item.clone()
                };

                let style = if i
                    == modal.get_current_list().get_selected_index()
                {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else if i == modal.get_current_list().get_items().len() - 1 {
                    // Special style for "Create new Profile"
                    Style::default().fg(Color::Green)
                } else if item.ends_with("(default)") {
                    // Special style for default profile
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                ListItem::new(Span::styled(content, style))
            })
            .collect();

        let title = match modal.current_tab {
            EditTab::Profiles => "Profiles",
            EditTab::Providers => "Providers",
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        let mut list_state = ListState::default();
        list_state.select(Some(modal.get_current_list().get_selected_index()));

        f.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_content(&self, f: &mut Frame, area: Rect, modal: &SettingsModal) {
        match modal.tab_focus {
            TabFocus::Settings | TabFocus::List => {
                self.render_settings(f, area, modal)
            }
            TabFocus::Creation => {
                if let Some(creator) = modal.get_current_creator() {
                    creator.render(f, area);
                }
            }
        }
    }

    fn render_settings(
        &self,
        f: &mut Frame,
        area: Rect,
        modal: &SettingsModal,
    ) {
        let settings = modal.get_current_settings_editor().get_settings();
        let mut items: Vec<ListItem> = settings
            .as_object()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, (key, value))| {
                let content = format!("{}: {}", key, value);
                let style = if i
                    == modal.get_current_settings_editor().get_current_field()
                    && modal.tab_focus == TabFocus::Settings
                {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else {
                    Style::default().bg(Color::Black).fg(Color::Cyan)
                };
                ListItem::new(Span::styled(content, style))
            })
            .collect();

        // Add new key input field if in AddingNewKey mode
        if matches!(
            modal.get_current_settings_editor().edit_mode,
            EditMode::AddingNewKey
        ) {
            let secure_indicator =
                if modal.get_current_settings_editor().is_new_value_secure() {
                    "🔒 "
                } else {
                    ""
                };
            items.push(ListItem::new(Span::styled(
                format!(
                    "{}New key: {}",
                    secure_indicator,
                    modal.get_current_settings_editor().get_new_key_buffer()
                ),
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )));
        }

        // Add new value input field if in AddingNewValue mode
        if matches!(
            modal.get_current_settings_editor().edit_mode,
            EditMode::AddingNewValue
        ) {
            let secure_indicator =
                if modal.get_current_settings_editor().is_new_value_secure() {
                    "🔒 "
                } else {
                    ""
                };
            items.push(ListItem::new(Span::styled(
                format!(
                    "{}{}: {}",
                    secure_indicator,
                    modal.get_current_settings_editor().get_new_key_buffer(),
                    modal.get_current_settings_editor().get_edit_buffer()
                ),
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )));
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Settings"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        if modal.tab_focus == TabFocus::Settings {
            list_state.select(Some(
                modal.get_current_settings_editor().get_current_field(),
            ));
        }

        f.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_instructions(
        &self,
        f: &mut Frame,
        area: Rect,
        modal: &SettingsModal,
    ) {
        let instructions = match (modal.current_tab, modal.tab_focus) {
            (EditTab::Profiles, TabFocus::List) => {
                "↑↓: Navigate | Enter: Select | R: Rename | D: Delete | Space: \
                 Set Default | Tab: Switch Tab"
            }
            (EditTab::Profiles, TabFocus::Settings) => {
                match modal.get_current_settings_editor().edit_mode {
                    EditMode::NotEditing => {
                        "↑↓: Navigate | Enter: Edit | n: New | N: New Secure | \
                         D: Delete | C: Clear | S: Show/Hide Secure | \
                         ←/Tab/q/Esc: Back to List"
                    }
                    EditMode::EditingValue => "Enter: Save | Esc: Cancel",
                    EditMode::AddingNewKey => {
                        "Enter: Confirm Key | Esc: Cancel"
                    }
                    EditMode::AddingNewValue => {
                        "Enter: Save New Value | Esc: Cancel"
                    }
                }
            }
            (EditTab::Profiles, TabFocus::Creation) => {
                "Enter: Create Profile | Esc: Cancel"
            }
            (EditTab::Providers, TabFocus::List) => {
                "↑↓: Navigate | Enter: Select | D: Delete | Tab: Switch Tab"
            }
            (EditTab::Providers, TabFocus::Settings) => {
                "↑↓: Navigate | Enter: Edit | D: Delete | Esc: Back to List"
            }
            (EditTab::Providers, TabFocus::Creation) => {
                "Enter: Create Provider | Esc: Cancel"
            }
        };

        let simple_string = SimpleString::from(instructions);
        let wrapped_instructions = simple_string.wrapped_spans(
            area.width as usize - 2, // Subtracting 2 for borders
            Some(Style::default().fg(Color::Cyan)),
            Some(" | "),
        );

        let wrapped_text: Vec<Line> =
            wrapped_instructions.into_iter().map(Line::from).collect();

        let paragraph = Paragraph::new(wrapped_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));

        f.render_widget(paragraph, area);
    }
}
