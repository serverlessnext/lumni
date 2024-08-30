use super::*;

pub struct ProfileList {
    profiles: Vec<UserProfile>,
    selected_index: usize,
    default_profile: Option<UserProfile>,
}

impl ProfileList {
    pub fn new(
        profiles: Vec<UserProfile>,
        default_profile: Option<UserProfile>,
    ) -> Self {
        ProfileList {
            profiles,
            selected_index: 0,
            default_profile,
        }
    }

    pub fn is_default_profile(&self, profile: &UserProfile) -> bool {
        self.default_profile
            .as_ref()
            .map_or(false, |default| default.id == profile.id)
    }

    pub fn get_items(&self) -> Vec<String> {
        let mut items: Vec<String> = self
            .profiles
            .iter()
            .map(|p| {
                if self.is_default_profile(p) {
                    format!("{} (default)", p.name)
                } else {
                    p.name.clone()
                }
            })
            .collect();

        items.push("Create new Profile".to_string());
        items
    }

    pub fn is_new_profile_selected(&self) -> bool {
        self.selected_index == self.profiles.len()
    }

    pub fn get_selected_profile(&self) -> Option<&UserProfile> {
        self.profiles.get(self.selected_index)
    }

    pub fn rename_selected_profile(&mut self, new_name: String) {
        if let Some(profile) = self.profiles.get_mut(self.selected_index) {
            profile.name = new_name;
        }
    }

    pub fn move_selection_up(&mut self) -> bool {
        let old_index = self.selected_index;
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            // Wrap to bottom
            self.selected_index = self.profiles.len();
        }
        old_index != self.selected_index
    }

    pub fn move_selection_down(&mut self) -> bool {
        let old_index = self.selected_index;
        if self.selected_index < self.profiles.len() {
            self.selected_index += 1;
        } else {
            // Wrap to top
            self.selected_index = 0;
        }
        old_index != self.selected_index
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

    pub fn add_profile(&mut self, profile: UserProfile) {
        self.profiles.push(profile);
        self.selected_index = self.profiles.len() - 1;
    }

    pub fn mark_as_default(&mut self, profile: &UserProfile) {
        self.default_profile = Some(profile.clone());
    }
}

impl GenericList for ProfileList {
    fn get_items(&self) -> Vec<String> {
        self.get_items()
    }

    fn get_selected_index(&self) -> usize {
        self.selected_index
    }
}
