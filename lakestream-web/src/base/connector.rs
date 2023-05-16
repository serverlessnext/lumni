use std::collections::HashMap;

use ::lakestream::{
    Config, FileObjectFilter, ListObjectsResult, ObjectStoreHandler,
};
use leptos::log;

#[derive(Clone)]
pub struct LakestreamHandler {
    handler: ObjectStoreHandler,
    config: Config,
}

impl LakestreamHandler {
    pub fn new(config: Config) -> Self {
        Self {
            handler: ObjectStoreHandler::new(None),
            config,
        }
    }

    pub async fn list_objects_demo(&self, _count: i32) -> Vec<String> {
        let recursive = false;
        let max_files = Some(20);
        let filter: Option<FileObjectFilter> = None;

        // TODO: get from user input
        let uri = "s3://abc".to_string();

        let callback = None;

        let result = self
            .handler
            .list_objects(
                &uri,
                &self.config,
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
            Ok(Some(ListObjectsResult::Buckets(buckets))) => {
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

pub fn get_config() -> Config {
    let mut config_hashmap = HashMap::new();

    let default_values = vec![
        ("AWS_ACCESS_KEY_ID", ""),
        ("AWS_SECRET_ACCESS_KEY", ""),
        ("AWS_REGION", "auto"),
        ("S3_ENDPOINT_URL", "https://localhost:8443"),
    ];

    for (key, default_value) in default_values.into_iter() {
        let key = key.to_string();
        // let value = load_data(&key).unwrap_or(default_value.to_string());
        // config_hashmap.insert(key, value);
        config_hashmap.insert(key, default_value.to_string());
    }

    // Create a Config instance
    Config {
        settings: config_hashmap,
    }
}
