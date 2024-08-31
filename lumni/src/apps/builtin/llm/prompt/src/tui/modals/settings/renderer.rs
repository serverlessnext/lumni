use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, Tabs,
};

use super::settings_editor::SettingsEditor;
use super::{
    EditMode, EditTab, ProviderConfig, SettingsModal, SimpleString, TabFocus,
    UserProfile,
};

pub struct SettingsRenderer;

impl SettingsRenderer {
    pub fn new() -> Self {
        SettingsRenderer
    }

    pub fn render_layout(
        &self,
        f: &mut Frame,
        area: Rect,
        modal: &SettingsModal,
        content_renderer: &dyn Fn(&mut Frame, Rect, &SettingsModal),
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
        content_renderer(f, main_chunks[1], modal);

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
                // check if item is default
                let content = if i
                    == modal.get_current_list().get_selected_index()
                    && modal.get_rename_buffer().is_some()
                {
                    format!("{}", modal.get_rename_buffer().unwrap())
                } else {
                    item.clone()
                };

                let style = if i
                    == modal.get_current_list().get_selected_index()
                {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else if i == modal.get_current_list().get_items().len() - 1 {
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
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(modal.get_current_list().get_selected_index()));

        f.render_stateful_widget(list, area, &mut list_state);
    }

    pub fn render_settings<T: SettingsItem>(
        &self,
        f: &mut Frame,
        area: Rect,
        item: Option<&T>,
        settings_editor: &SettingsEditor,
    ) {
        if let Some(item) = item {
            let settings = settings_editor.get_settings();
            let mut items: Vec<ListItem> = settings
                .as_object()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, (key, value))| {
                    let is_editable = !key.starts_with("__");
                    let display_value =
                        settings_editor.get_display_value(value);

                    let content = if settings_editor.edit_mode
                        == EditMode::EditingValue
                        && i == settings_editor.get_current_field()
                        && is_editable
                    {
                        format!(
                            "{}: {}",
                            key,
                            settings_editor.get_edit_buffer()
                        )
                    } else {
                        format!("{}: {}", key, display_value)
                    };

                    let style = if i == settings_editor.get_current_field() {
                        Style::default()
                            .bg(Color::Rgb(40, 40, 40))
                            .fg(Color::White)
                    } else if is_editable {
                        Style::default().bg(Color::Black).fg(Color::Cyan)
                    } else {
                        Style::default().bg(Color::Black).fg(Color::DarkGray)
                    };
                    ListItem::new(Line::from(vec![Span::styled(
                        content, style,
                    )]))
                })
                .collect();

            // Add new key input field if in AddingNewKey mode
            if settings_editor.edit_mode == EditMode::AddingNewKey {
                let secure_indicator = if settings_editor.is_new_value_secure()
                {
                    "🔒 "
                } else {
                    ""
                };
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!(
                        "{}New key: {}",
                        secure_indicator,
                        settings_editor.get_new_key_buffer()
                    ),
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                )])));
            }

            // Add new value input field if in AddingNewValue mode
            if settings_editor.edit_mode == EditMode::AddingNewValue {
                let secure_indicator = if settings_editor.is_new_value_secure()
                {
                    "🔒 "
                } else {
                    ""
                };
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!(
                        "{}{}: {}",
                        secure_indicator,
                        settings_editor.get_new_key_buffer(),
                        settings_editor.get_edit_buffer()
                    ),
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                )])));
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "{} Settings: {}",
                    T::item_type(),
                    item.name()
                )))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");

            let mut state = ListState::default();
            state.select(Some(settings_editor.get_current_field()));

            f.render_stateful_widget(list, area, &mut state);
        } else {
            let paragraph =
                Paragraph::new(format!("No {} selected", T::item_type()))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("{} Settings", T::item_type())),
                    );
            f.render_widget(paragraph, area);
        }
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
            (EditTab::Providers, TabFocus::List) => {
                "↑↓: Navigate | Enter: Select | R: Rename | D: Delete | Tab: \
                 Switch Tab"
            }
            (_, TabFocus::Settings) => match modal
                .get_current_settings_editor()
                .edit_mode
            {
                EditMode::NotEditing => {
                    "↑↓: Navigate | Enter: Edit | n: New | N: New Secure | D: \
                     Delete | C: Clear | S: Show/Hide Secure | ←/Tab/q/Esc: \
                     Back to List"
                }
                EditMode::EditingValue => "Enter: Save | Esc: Cancel",
                EditMode::AddingNewKey => "Enter: Confirm Key | Esc: Cancel",
                EditMode::AddingNewValue => {
                    "Enter: Save New Value | Esc: Cancel"
                }
            },
            (_, TabFocus::Creation) => "Enter: Create | Esc: Cancel",
        };

        let simple_string = SimpleString::from(instructions);
        let wrapped_spans = simple_string.wrapped_spans(
            area.width as usize - 2, // Subtract 2 for left and right borders
            Some(Style::default().fg(Color::Cyan)),
            Some(" | "),
        );

        let wrapped_text: Vec<Line> =
            wrapped_spans.into_iter().map(Line::from).collect();

        let paragraph = Paragraph::new(wrapped_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));

        f.render_widget(paragraph, area);
    }
}

pub trait SettingsItem {
    fn name(&self) -> &str;
    fn item_type() -> &'static str;
}

impl SettingsItem for UserProfile {
    fn name(&self) -> &str {
        &self.name
    }

    fn item_type() -> &'static str {
        "Profile"
    }
}

impl SettingsItem for ProviderConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn item_type() -> &'static str {
        "Provider"
    }
}
