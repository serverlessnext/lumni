use super::*;

pub trait ListItemTrait: Clone {
    fn name(&self) -> &str;
    fn id(&self) -> i64;
    fn with_new_name(&self, new_name: String) -> Self;
    fn item_type() -> &'static str
    where
        Self: Sized;
}

pub struct SettingsList<T: ListItemTrait> {
    items: Vec<T>,
    selected_index: usize,
    pub default_item: Option<T>,
}

impl<T: ListItemTrait> SettingsList<T> {
    pub fn new(items: Vec<T>, default_item: Option<T>) -> Self {
        let mut list = SettingsList {
            items,
            selected_index: 0,
            default_item: None,
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
        items.push(format!("Create new {}", T::item_type()));
        items
    }
}

impl<T: ListItemTrait + SettingsItem> SettingsListTrait for SettingsList<T> {
    type Item = T;

    fn get_items(&self) -> Vec<String> {
        self.get_items()
    }

    fn get_selected_index(&self) -> usize {
        self.selected_index
    }

    fn get_selected_item(&self) -> Option<&Self::Item> {
        self.items.get(self.selected_index)
    }
}

pub trait SettingsListTrait {
    type Item: ListItemTrait + SettingsItem;
    fn get_items(&self) -> Vec<String>;
    fn get_selected_index(&self) -> usize;
    fn get_selected_item(&self) -> Option<&Self::Item>;
}

impl ListItemTrait for UserProfile {
    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn with_new_name(&self, new_name: String) -> Self {
        UserProfile {
            name: new_name,
            ..self.clone()
        }
    }

    fn item_type() -> &'static str {
        "Profile"
    }
}

impl ListItemTrait for ProviderConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> i64 {
        self.id.unwrap_or(0)
    }

    fn with_new_name(&self, new_name: String) -> Self {
        ProviderConfig {
            name: new_name,
            ..self.clone()
        }
    }

    fn item_type() -> &'static str {
        "Provider"
    }
}
