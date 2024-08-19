use super::*;

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
                let content = if i
                    == profile_edit_modal.profile_list.get_selected_index()
                    && matches!(
                        profile_edit_modal.ui_state.edit_mode,
                        EditMode::RenamingProfile
                    ) {
                    profile_edit_modal
                        .new_profile_name
                        .as_ref()
                        .unwrap_or(profile)
                } else {
                    profile
                };
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
                let is_secure = value.is_object()
                    && value.get("was_encrypted")
                        == Some(&serde_json::Value::Bool(true));
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
                    let display_value = if is_secure {
                        if profile_edit_modal.settings_editor.is_show_secure() {
                            value["value"].as_str().unwrap_or("").to_string()
                        } else {
                            "*****".to_string()
                        }
                    } else {
                        value.as_str().unwrap_or("").to_string()
                    };
                    let lock_icon = if is_secure {
                        if profile_edit_modal.settings_editor.is_show_secure() {
                            "🔓 "
                        } else {
                            "🔒 "
                        }
                    } else {
                        ""
                    };
                    let empty_indicator = if display_value.is_empty() {
                        " (empty)"
                    } else {
                        ""
                    };
                    format!(
                        "{}{}: {}{}",
                        lock_icon, key, display_value, empty_indicator
                    )
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
            (Focus::NewProfileCreation, _) => profile_edit_modal
                .ui_state
                .new_profile_creator
                .as_ref()
                .map(|creator| creator.get_instructions())
                .unwrap_or(""),
            _ => "",
        };
        let paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(paragraph, area);
    }
}
