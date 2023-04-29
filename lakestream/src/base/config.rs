use std::collections::HashMap;

#[derive(Clone, Default, Debug)]
pub struct Config {
    pub settings: HashMap<String, String>,
}

impl Config {
    pub fn new(settings: HashMap<String, String>) -> Config {
        Config { settings }
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
}
