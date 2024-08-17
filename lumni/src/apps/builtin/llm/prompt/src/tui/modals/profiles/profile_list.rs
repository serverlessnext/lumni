use super::*;

pub struct ProfileList {
    profiles: Vec<String>,
    selected_index: usize,
}

impl ProfileList {
    pub fn new(profiles: Vec<String>) -> Self {
        ProfileList {
            profiles,
            selected_index: 0,
        }
    }

    pub fn get_selected_profile(&self) -> Option<&str> {
        self.profiles.get(self.selected_index).map(|s| s.as_str())
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
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

    pub fn get_profiles(&self) -> &[String] {
        &self.profiles
    }

    pub fn add_profile(&mut self, name: String) {
        self.profiles.push(name);
        self.selected_index = self.profiles.len() - 1;
    }

    pub fn get_selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn is_new_profile_selected(&self) -> bool {
        self.selected_index == self.profiles.len()
    }

    pub fn total_items(&self) -> usize {
        self.profiles.len() + 1 // +1 for "New Profile" option
    }
}
