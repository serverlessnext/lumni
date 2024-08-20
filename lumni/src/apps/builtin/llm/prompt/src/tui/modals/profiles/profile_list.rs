use super::*;

pub struct ProfileList {
    profiles: Vec<UserProfile>,
    selected_index: usize,
    default_profile: Option<UserProfile>,
}

impl ProfileList {
    pub fn new(profiles: Vec<UserProfile>, default_profile: Option<UserProfile>) -> Self {
        ProfileList {
            profiles,
            selected_index: 0,
            default_profile,
        }
    }

    pub fn get_selected_profile(&self) -> Option<&UserProfile> {
        self.profiles.get(self.selected_index)
    }

    pub fn select_new_profile(&mut self) {
        self.selected_index = self.profiles.len();
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else if self.selected_index == 0 && !self.profiles.is_empty() {
            // If at the top and "New Profile" is selected, wrap to the bottom
            self.selected_index = self.profiles.len() - 1;
        }
    }

    pub fn is_new_profile_selected(&self) -> bool {
        self.selected_index == self.profiles.len()
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.profiles.len() {
            self.selected_index += 1;
        }
    }

    pub async fn rename_profile(
        &mut self,
        new_name: String,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(profile) = self.profiles.get(self.selected_index) {
            db_handler.rename_profile(profile, &new_name).await?;
            self.profiles[self.selected_index].name = new_name;
        }
        Ok(())
    }

    pub async fn delete_profile(
        &mut self,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(profile_name) = self.profiles.get(self.selected_index) {
            db_handler.delete_profile(profile_name).await?;
            self.profiles.remove(self.selected_index);
            if self.selected_index >= self.profiles.len()
                && !self.profiles.is_empty()
            {
                self.selected_index = self.profiles.len() - 1;
            }
        }
        Ok(())
    }

    // TODO: this does not update the database
    pub fn start_renaming(&self) -> String {
        let profile = self.profiles
            .get(self.selected_index);
        if let Some(profile) = profile {
            profile.name.clone()
        } else {
            "".to_string()
        }
    }

    pub fn add_profile(&mut self, profile: UserProfile) {
        self.profiles.push(profile);
        self.selected_index = self.profiles.len() - 1;
    }

    pub fn get_selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn total_items(&self) -> usize {
        self.profiles.len() + 1 // +1 for "New Profile" option
    }

    pub fn mark_as_default(&mut self, profile: &UserProfile) {
        self.default_profile = Some(profile.clone());
    }

    pub fn is_default_profile(&self, profile: &UserProfile) -> bool {
        self.default_profile
            .as_ref()
            .map_or(false, |default| default == profile)
    }

    pub fn get_profiles(&self) -> Vec<String> {
        self.profiles
            .iter()
            .map(|p| {
                if self.is_default_profile(p) {
                    format!("* {}", p.name) // Prepend an asterisk to mark the default profile
                } else {
                    p.name.clone()
                }
            })
            .collect()
    }
}
