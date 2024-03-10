use std::sync::{Arc, Mutex};

use localencrypt::LocalEncrypt;

use crate::components::forms::{FormStorageHandler, MemoryStorage};

const DEFAULT_HOME_URL: &str = "/console";

#[derive(Clone)]
pub struct GlobalState {
    pub store: Arc<Mutex<FormStorageHandler<MemoryStorage>>>,
    pub vault: Option<LocalEncrypt>,
    pub runtime: Option<RunTime>,
}

impl GlobalState {
    fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(FormStorageHandler::new(
                MemoryStorage::new(),
            ))),
            vault: None,
            runtime: None,
        }
    }

    pub fn is_vault_initialized(&self) -> bool {
        self.vault.is_some()
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct RunTime {
    previous_url: String,
}

impl Default for RunTime {
    fn default() -> Self {
        Self::new()
    }
}

impl RunTime {
    pub fn new() -> Self {
        Self {
            previous_url: DEFAULT_HOME_URL.to_string(),
        }
    }

    pub fn previous_url(&self) -> &String {
        &self.previous_url
    }

    pub fn set_previous_url(&mut self, url: String) {
        self.previous_url = url;
    }
}
