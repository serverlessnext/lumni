use ::xlatti::{
    EnvironmentConfig, FileObjectFilter, ListObjectsResult, ObjectStoreHandler,
};
use leptos::{ev::select, log};

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

    pub async fn list_objects(&self, uri: String, count: u32) -> Vec<String> {
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

        match result {
            Ok(Some(ListObjectsResult::FileObjects(file_objects))) => {
                file_objects
                    .into_iter()
                    .map(|fo| fo.name().to_owned())
                    .collect::<Vec<_>>()
            }
            Ok(Some(ListObjectsResult::RowItems(buckets))) => {
                // note - CORS does not work on Bucket List
                buckets
                    .into_iter()
                    .map(|bucket| bucket.name().to_owned())
                    .collect::<Vec<_>>()
            }
            Err(err) => {
                log!("Error: {:?}", err);
                vec![]
            }
            _ => vec![],
        }
    }
}
