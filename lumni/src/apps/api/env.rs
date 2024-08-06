use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ApplicationEnv {
    config_dir: Option<PathBuf>,
    prog_name: Option<String>,
}

impl ApplicationEnv {
    pub fn new() -> ApplicationEnv {
        ApplicationEnv {
            config_dir: None,
            prog_name: None,
        }
    }

    pub fn set_config_dir(&mut self, config_dir: PathBuf) {
        self.config_dir = Some(config_dir);
    }

    pub fn get_config_dir(&self) -> Option<&PathBuf> {
        self.config_dir.as_ref()
    }

    pub fn set_prog_name(&mut self, program_name: String) {
        self.prog_name = Some(program_name);
    }

    pub fn get_prog_name(&self) -> Option<&String> {
        self.prog_name.as_ref()
    }
}
