use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    ProfileList,
    SettingsList,
    NewProfileCreation,
    RenamingProfile,
}

// Update the EditMode enum to remove the CreatingNewProfile variant
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    NotEditing,
    EditingValue,
    AddingNewKey,
    AddingNewValue,
    RenamingProfile,
}

// Update the UIState struct to include the new_profile_creator field
#[derive(Debug)]
pub struct UIState {
    pub focus: Focus,
    pub edit_mode: EditMode,
    pub show_secure: bool,
    pub new_profile_creator: Option<NewProfileCreator>,
}

impl UIState {
    pub fn new() -> Self {
        UIState {
            focus: Focus::ProfileList,
            edit_mode: EditMode::NotEditing,
            show_secure: false,
            new_profile_creator: None,
        }
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
    }

    pub fn set_edit_mode(&mut self, mode: EditMode) {
        self.edit_mode = mode;
    }

    pub fn start_new_profile_creation(
        &mut self,
        db_handler: UserProfileDbHandler,
    ) {
        self.new_profile_creator = Some(NewProfileCreator::new(db_handler));
        self.focus = Focus::NewProfileCreation;
    }

    pub fn cancel_new_profile_creation(&mut self) {
        self.new_profile_creator = None;
        self.focus = Focus::ProfileList;
    }
}
