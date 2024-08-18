use super::{ModelServer, ModelSpec, ServerManager};
use crate::apps::builtin::llm::prompt::src::chat::db::ModelServerName;

pub struct ModelBackend {
    pub server: ModelServer,
    pub model: Option<ModelSpec>,
}

impl ModelBackend {
    pub fn server_name(&self) -> ModelServerName {
        self.server.server_name()
    }
}
