use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use super::profile_edit_modal::{EditMode, Focus, ProfileEditModal};

pub struct ProfileEditRenderer;

impl ProfileEditRenderer {
    pub fn new() -> Self {
        ProfileEditRenderer
    }

    pub fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("Profile Editor")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        f.render_widget(title, area);
    }

    pub fn render_profile_list(
        &self,
        f: &mut Frame,
        area: Rect,
        profile_edit_modal: &ProfileEditModal,
    ) {
        let profiles = profile_edit_modal.profile_list.get_profiles();
        let mut items: Vec<ListItem> = profiles
            .iter()
            .enumerate()
            .map(|(i, profile)| {
                let content = profile;
                let style = if i
                    == profile_edit_modal.profile_list.get_selected_index()
                    && matches!(
                        profile_edit_modal.ui_state.focus,
                        Focus::ProfileList | Focus::RenamingProfile
                    ) {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else {
                    Style::default().bg(Color::Black).fg(Color::Cyan)
                };
                ListItem::new(Line::from(vec![Span::styled(content, style)]))
            })
            .collect();

        // Add "New Profile" option
        let new_profile_style = if profile_edit_modal
            .profile_list
            .is_new_profile_selected()
            && matches!(profile_edit_modal.ui_state.focus, Focus::ProfileList)
        {
            Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
        } else {
            Style::default().bg(Color::Black).fg(Color::Green)
        };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "+ New Profile",
            new_profile_style,
        )])));

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Profiles"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let mut state = ListState::default();
        state
            .select(Some(profile_edit_modal.profile_list.get_selected_index()));

        f.render_stateful_widget(list, area, &mut state);
    }

    pub fn render_settings_list(
        &self,
        f: &mut Frame,
        area: Rect,
        profile_edit_modal: &ProfileEditModal,
    ) {
        let settings = profile_edit_modal.settings_editor.get_settings();
        let mut items: Vec<ListItem> = settings
            .as_object()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, (key, value))| {
                let is_editable = !key.starts_with("__");
                let display_value =
                    profile_edit_modal.settings_editor.get_display_value(value);

                let content = if matches!(
                    profile_edit_modal.ui_state.edit_mode,
                    EditMode::EditingValue
                ) && i
                    == profile_edit_modal.settings_editor.get_current_field()
                    && is_editable
                {
                    format!(
                        "{}: {}",
                        key,
                        profile_edit_modal.settings_editor.get_edit_buffer()
                    )
                } else {
                    format!("{}: {}", key, display_value)
                };

                let style = if i
                    == profile_edit_modal.settings_editor.get_current_field()
                    && matches!(
                        profile_edit_modal.ui_state.focus,
                        Focus::SettingsList
                    ) {
                    Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else if is_editable {
                    Style::default().bg(Color::Black).fg(Color::Cyan)
                } else {
                    Style::default().bg(Color::Black).fg(Color::DarkGray)
                };
                ListItem::new(Line::from(vec![Span::styled(content, style)]))
            })
            .collect();

        // Add new key input field if in AddingNewKey mode
        if matches!(
            profile_edit_modal.ui_state.edit_mode,
            EditMode::AddingNewKey
        ) {
            let secure_indicator =
                if profile_edit_modal.settings_editor.is_new_value_secure() {
                    "🔒 "
                } else {
                    ""
                };
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!(
                    "{}New key: {}",
                    secure_indicator,
                    profile_edit_modal.settings_editor.get_new_key_buffer()
                ),
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )])));
        }

        // Add new value input field if in AddingNewValue mode
        if matches!(
            profile_edit_modal.ui_state.edit_mode,
            EditMode::AddingNewValue
        ) {
            let secure_indicator =
                if profile_edit_modal.settings_editor.is_new_value_secure() {
                    "🔒 "
                } else {
                    ""
                };
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!(
                    "{}{}: {}",
                    secure_indicator,
                    profile_edit_modal.settings_editor.get_new_key_buffer(),
                    profile_edit_modal.settings_editor.get_edit_buffer()
                ),
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White),
            )])));
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Settings"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let mut state = ListState::default();
        state.select(Some(
            profile_edit_modal.settings_editor.get_current_field(),
        ));

        f.render_stateful_widget(list, area, &mut state);
    }

    pub fn render_instructions(
        &self,
        f: &mut Frame,
        area: Rect,
        profile_edit_modal: &ProfileEditModal,
    ) {
        let instructions = match (
            &profile_edit_modal.ui_state.focus,
            &profile_edit_modal.ui_state.edit_mode,
        ) {
            (Focus::ProfileList, EditMode::NotEditing) => {
                "↑↓: Navigate | Enter: Select/Create | R: Rename | D: Delete | \
                 Space: Set Default | →/Tab: Settings | Esc: Close"
            }
            (Focus::RenamingProfile, EditMode::RenamingProfile) => {
                "Enter: Confirm Rename | Esc: Cancel"
            }
            (Focus::SettingsList, EditMode::NotEditing) => {
                "↑↓: Navigate | Enter: Edit | n: New | N: New Secure | D: \
                 Delete | C: Clear | S: Show/Hide Secure | ←/Tab/q/Esc: \
                 Profiles"
            }
            (Focus::SettingsList, EditMode::EditingValue) => {
                "Enter: Save | Esc: Cancel"
            }
            (Focus::SettingsList, EditMode::AddingNewKey) => {
                "Enter: Confirm Key | Esc: Cancel"
            }
            (Focus::SettingsList, EditMode::AddingNewValue) => {
                "Enter: Save New Value | Esc: Cancel"
            }
            (Focus::ProfileCreation, _) => {
                "Follow on-screen instructions to create a new profile"
            }
            _ => "",
        };

        let paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));
        f.render_widget(paragraph, area);
    }

    pub fn render_layout(
        &self,
        f: &mut Frame,
        area: Rect,
        profile_edit_modal: &ProfileEditModal,
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

        self.render_profile_list(f, main_chunks[0], profile_edit_modal);
        self.render_settings_list(f, main_chunks[1], profile_edit_modal);

        self.render_instructions(f, chunks[2], profile_edit_modal);
    }
}
