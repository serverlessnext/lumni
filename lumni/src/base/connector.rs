use crate::{EnvironmentConfig, InternalError, ObjectStoreHandler, Table};

#[derive(Clone)]
pub struct LumniHandler {
    handler: ObjectStoreHandler,
    config: EnvironmentConfig,
}

impl LumniHandler {
    pub fn new(config: EnvironmentConfig) -> Self {
        Self {
            handler: ObjectStoreHandler::new(None),
            config,
        }
    }

    pub async fn execute_query(
        &self,
        query: String,
    ) -> Result<Box<dyn Table>, InternalError> {
        let callback = None;
        let result = self
            .handler
            .execute_query(&query, &self.config, callback)
            .await;
        result
    }
}
