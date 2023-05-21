use crate::StringVault;

const DEFAULT_HOME_URL: &str = "/home";

#[derive(Clone, Default)]
pub struct GlobalState {
    pub vault: Option<StringVault>,
    pub runtime: Option<RunTime>,
}

#[derive(Clone, Debug)]
pub struct RunTime {
    previous_url: String,
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
