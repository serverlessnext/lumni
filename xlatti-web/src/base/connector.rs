use ::xlatti::{EnvironmentConfig, LakestreamError, ObjectStoreHandler, Table};

#[derive(Clone)]
pub struct LakestreamHandler {
    handler: ObjectStoreHandler,
    config: EnvironmentConfig,
}

impl LakestreamHandler {
    pub fn new(config: EnvironmentConfig) -> Self {
        Self {
            handler: ObjectStoreHandler::new(None),
            config,
        }
    }

    pub async fn execute_query(
        &self,
        query: String,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        let callback = None;
        let result = self
            .handler
            .execute_query(&query, &self.config, callback)
            .await;
        result
    }
}
