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
        skip_hidden: bool,
        recursive: bool,
    ) -> Result<Box<dyn Table>, InternalError> {
        let callback = None;
        let result = self
            .handler
            .execute_query(
                &query,
                &self.config,
                skip_hidden,
                recursive,
                None,
                callback,
            )
            .await;
        result
    }
}
