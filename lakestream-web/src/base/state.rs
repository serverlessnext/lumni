use std::cell::RefCell;
use std::rc::Rc;

use crate::stringvault::StringVault;

const DEFAULT_HOME_URL: &str = "/home";

#[derive(Clone, Default)]
pub struct GlobalState {
    pub vault: Option<Rc<RefCell<StringVault>>>,
    pub runtime: Option<RunTime>,
}

#[derive(Clone, Debug)]
pub struct RunTime {
    previous_url: String,
    vault_initialized: bool,
}

impl RunTime {
    pub fn new() -> Self {
        Self {
            previous_url: DEFAULT_HOME_URL.to_string(),
            vault_initialized: false,
        }
    }

    pub fn previous_url(&self) -> &String {
        &self.previous_url
    }

    pub fn set_previous_url(&mut self, url: String) {
        self.previous_url = url;
    }

    pub fn vault_initialized(&self) -> bool {
        self.vault_initialized
    }

    pub fn set_vault_initialized(&mut self, initialized: bool) {
        self.vault_initialized = initialized;
    }
}
