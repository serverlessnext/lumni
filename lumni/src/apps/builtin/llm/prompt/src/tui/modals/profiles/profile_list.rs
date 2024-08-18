use super::*;

pub struct ProfileList {
    profiles: Vec<String>,
    selected_index: usize,
    default_profile: Option<String>,
}

impl ProfileList {
    pub fn new(profiles: Vec<String>) -> Self {
        ProfileList {
            profiles,
            selected_index: 0,
            default_profile: None,
        }
    }

    pub fn get_selected_profile(&self) -> Option<&str> {
        self.profiles.get(self.selected_index).map(|s| s.as_str())
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
        if let Some(old_name) = self.profiles.get(self.selected_index) {
            db_handler.rename_profile(old_name, &new_name).await?;
            self.profiles[self.selected_index] = new_name;
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

    pub fn start_renaming(&self) -> String {
        self.profiles
            .get(self.selected_index)
            .cloned()
            .unwrap_or_default()
    }

    pub fn add_profile(&mut self, name: String) {
        self.profiles.push(name);
        self.selected_index = self.profiles.len() - 1;
    }

    pub fn get_selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn total_items(&self) -> usize {
        self.profiles.len() + 1 // +1 for "New Profile" option
    }

    pub fn mark_as_default(&mut self, profile: &str) {
        self.default_profile = Some(profile.to_string());
    }

    pub fn get_profiles(&self) -> Vec<String> {
        self.profiles
            .iter()
            .map(|p| {
                if Some(p) == self.default_profile.as_ref() {
                    format!("* {}", p) // Prepend an asterisk to mark the default profile
                } else {
                    p.clone()
                }
            })
            .collect()
    }
}
