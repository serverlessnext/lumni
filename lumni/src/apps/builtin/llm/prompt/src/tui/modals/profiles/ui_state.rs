#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    ProfileList,
    SettingsList,
    NewProfileType,
    RenamingProfile,
    ModelSelection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    NotEditing,
    EditingValue,
    AddingNewKey,
    AddingNewValue,
    CreatingNewProfile,
    RenamingProfile,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UIState {
    pub focus: Focus,
    pub edit_mode: EditMode,
    pub show_secure: bool,
}

impl UIState {
    pub fn new() -> Self {
        UIState {
            focus: Focus::ProfileList,
            edit_mode: EditMode::NotEditing,
            show_secure: false,
        }
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
    }

    pub fn set_edit_mode(&mut self, mode: EditMode) {
        self.edit_mode = mode;
    }
}
