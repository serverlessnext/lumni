use std::collections::HashMap;

#[derive(Clone, Default, Debug)]
pub struct EnvironmentConfig {
    pub settings: HashMap<String, String>,
}

impl EnvironmentConfig {
    pub fn new(settings: HashMap<String, String>) -> EnvironmentConfig {
        EnvironmentConfig { settings }
    }

    // shortcut method to create a Config with a single key-value pair
    pub fn with_setting(key: String, value: String) -> EnvironmentConfig {
        let mut settings = HashMap::new();
        settings.insert(key, value);
        EnvironmentConfig { settings }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.settings.contains_key(key)
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }

    pub fn get_settings(&self) -> &HashMap<String, String> {
        &self.settings
    }
}
