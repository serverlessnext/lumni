use super::*;

pub trait ListItemTrait: Clone {
    fn name(&self) -> &str;
    fn id(&self) -> i64;
    fn with_new_name(&self, new_name: String) -> Self;
}

#[derive(Debug)]
pub struct SettingsList<T: ListItemTrait> {
    pub items: Vec<T>,
    pub selected_index: usize,
    pub default_item: Option<T>,
    item_type: String,
}

impl<T: ListItemTrait> SettingsList<T> {
    pub fn new(
        items: Vec<T>,
        default_item: Option<T>,
        item_type: String,
    ) -> Self {
        let mut list = SettingsList {
            items,
            selected_index: 0,
            default_item: None,
            item_type,
        };
        if let Some(default) = default_item {
            list.mark_as_default(&default);
        }
        list
    }

    pub fn new_with_selected_item(
        items: Vec<T>,
        default_item: Option<T>,
        item_type: String,
        list_item_id: i64,
    ) -> Self {
        let selected_index = items
            .iter()
            .position(|item| item.id() == list_item_id)
            .unwrap_or(0);

        let mut list = SettingsList {
            items,
            selected_index,
            default_item: None,
            item_type,
        };

        if let Some(default) = default_item {
            list.mark_as_default(&default);
        }

        list
    }

    pub fn get_selected_item(&self) -> Option<&T> {
        self.items.get(self.selected_index)
    }

    pub fn is_new_item_selected(&self) -> bool {
        self.selected_index == self.items.len()
    }

    pub fn move_selection_up(&mut self) -> bool {
        let old_index = self.selected_index;
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            // Wrap to bottom
            self.selected_index = self.items.len();
        }
        old_index != self.selected_index
    }

    pub fn move_selection_down(&mut self) -> bool {
        let old_index = self.selected_index;
        if self.selected_index < self.items.len() {
            self.selected_index += 1;
        } else {
            // Wrap to top
            self.selected_index = 0;
        }
        old_index != self.selected_index
    }

    pub fn add_item(&mut self, item: T) {
        self.items.push(item);
        self.selected_index = self.items.len() - 1;
    }

    pub fn rename_selected_item(&mut self, new_name: String) {
        if let Some(item) = self.items.get_mut(self.selected_index) {
            *item = item.with_new_name(new_name);
        }
    }

    pub fn remove_selected_item(&mut self) {
        if self.selected_index < self.items.len() {
            self.items.remove(self.selected_index);
            if self.selected_index >= self.items.len() && !self.items.is_empty()
            {
                self.selected_index = self.items.len() - 1;
            }
        }
    }

    pub fn mark_as_default(&mut self, item: &T) {
        self.default_item = Some(item.clone());
    }

    pub fn is_default_item(&self, item: &T) -> bool {
        self.default_item
            .as_ref()
            .map_or(false, |default| default.id() == item.id())
    }

    pub fn get_items(&self) -> Vec<String> {
        let mut items: Vec<String> = self
            .items
            .iter()
            .map(|item| {
                if self.is_default_item(item) {
                    format!("{} (default)", item.name())
                } else {
                    item.name().to_string()
                }
            })
            .collect();
        items.push(format!("Create new {}", self.item_type));
        items
    }
}

impl<T: ListItemTrait + SettingsItem> SettingsListTrait for SettingsList<T> {
    type Item = T;

    fn get_selected_index(&self) -> usize {
        self.selected_index
    }
}

pub trait SettingsListTrait {
    type Item: ListItemTrait + SettingsItem;
    fn get_selected_index(&self) -> usize;
}

impl ListItemTrait for ConfigItem {
    fn name(&self) -> &str {
        match self {
            ConfigItem::UserProfile(profile) => &profile.name,
            ConfigItem::DatabaseConfig(config) => &config.name,
        }
    }

    fn id(&self) -> i64 {
        match self {
            ConfigItem::UserProfile(profile) => profile.id,
            ConfigItem::DatabaseConfig(config) => config.id,
        }
    }

    fn with_new_name(&self, new_name: String) -> Self {
        match self {
            ConfigItem::UserProfile(profile) => {
                ConfigItem::UserProfile(UserProfile {
                    id: profile.id,
                    name: new_name,
                })
            }
            ConfigItem::DatabaseConfig(config) => {
                ConfigItem::DatabaseConfig(DatabaseConfigurationItem {
                    id: config.id,
                    name: new_name,
                    section: config.section.clone(),
                })
            }
        }
    }
}
