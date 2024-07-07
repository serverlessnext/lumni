use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ApplicationEnv {
    config_dir: Option<PathBuf>,
}

impl ApplicationEnv {
    pub fn new() -> ApplicationEnv {
        ApplicationEnv { config_dir: None }
    }

    pub fn set_config_dir(&mut self, config_dir: PathBuf) {
        self.config_dir = Some(config_dir);
    }

    pub fn get_config_dir(&self) -> Option<&PathBuf> {
        self.config_dir.as_ref()
    }
}
