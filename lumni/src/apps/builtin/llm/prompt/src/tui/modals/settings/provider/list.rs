use super::{
    ApplicationError, GenericList, ProviderConfig, UserProfileDbHandler,
};

pub struct ProviderList {
    providers: Vec<ProviderConfig>,
    selected_index: usize,
}

impl ProviderList {
    pub fn new(providers: Vec<ProviderConfig>) -> Self {
        ProviderList {
            providers,
            selected_index: 0,
        }
    }

    pub fn get_selected_provider(&self) -> Option<&ProviderConfig> {
        self.providers.get(self.selected_index)
    }

    pub fn is_new_provider_selected(&self) -> bool {
        self.selected_index == self.providers.len()
    }

    pub fn move_selection_up(&mut self) -> bool {
        let old_index = self.selected_index;
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            // Wrap to bottom
            self.selected_index = self.providers.len();
        }
        old_index != self.selected_index
    }

    pub fn move_selection_down(&mut self) -> bool {
        let old_index = self.selected_index;
        if self.selected_index < self.providers.len() {
            self.selected_index += 1;
        } else {
            // Wrap to top
            self.selected_index = 0;
        }
        old_index != self.selected_index
    }

    pub async fn delete_provider(
        &mut self,
        db_handler: &mut UserProfileDbHandler,
    ) -> Result<(), ApplicationError> {
        if let Some(provider) = self.providers.get(self.selected_index) {
            if let Some(id) = provider.id {
                db_handler.delete_provider_config(id).await?;
                self.providers.remove(self.selected_index);
                if self.selected_index >= self.providers.len()
                    && !self.providers.is_empty()
                {
                    self.selected_index = self.providers.len() - 1;
                }
            }
        }
        Ok(())
    }

    pub fn add_provider(&mut self, provider: ProviderConfig) {
        self.providers.push(provider);
        self.selected_index = self.providers.len() - 1;
    }

    pub fn rename_selected_provider(&mut self, new_name: String) {
        if let Some(provider) = self.providers.get_mut(self.selected_index) {
            provider.name = new_name;
        }
    }
}

impl GenericList for ProviderList {
    fn get_items(&self) -> Vec<String> {
        let mut items: Vec<String> =
            self.providers.iter().map(|p| p.name.clone()).collect();
        items.push("Create new Provider".to_string());
        items
    }

    fn get_selected_index(&self) -> usize {
        self.selected_index
    }
}
