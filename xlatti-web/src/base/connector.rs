use ::xlatti::{
    EnvironmentConfig, FileObjectFilter, LakestreamError, ObjectStoreHandler,
    Table,
};
use leptos::ev::select;
use leptos::log;

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

    pub async fn list_objects(
        &self,
        uri: String,
        count: u32,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        let recursive = false;
        let max_files = Some(count);
        let filter: Option<FileObjectFilter> = None;
        let callback = None;

        // TODO: implement column selection
        let selected_columns = None;

        let result = self
            .handler
            .list_objects(
                &uri,
                &self.config,
                selected_columns,
                recursive,
                max_files,
                &filter,
                callback,
            )
            .await;
        log::debug!("Web Results: {:?}", result);
        result
    }
}
