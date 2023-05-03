use async_trait::async_trait;
use log::info;

use crate::utils::backend_parse::object_stores_from_config;
use crate::utils::uri_parse::ParsedUri;
use crate::{
    CallbackWrapper, Config, FileObjectFilter, LakestreamError,
    ListObjectsResult, ObjectStore,
};

pub struct ObjectStoreHandler {}

impl ObjectStoreHandler {
    pub fn new(_configs: Option<Vec<Config>>) -> Self {
        // creating with config will be used in future
        ObjectStoreHandler {}
    }

    pub async fn list_objects(
        &self,
        uri: String,
        config: Config,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<CallbackWrapper>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(&uri);
        if let Some(bucket) = &parsed_uri.bucket {
            // list files in a bucket
            info!("Listing files in bucket {}", bucket);
            self.list_files_in_bucket(
                parsed_uri, config, recursive, max_files, filter, callback,
            )
            .await
        } else {
            // list buckets
            if callback.is_some() {
                panic!("Listing buckets not yet supported with callback");
            }
            info!("Listing buckets");
            // Clone the original config and update the settings
            // will change the input config to reference at future update
            let mut updated_config = config.clone();
            updated_config.settings.insert(
                "uri".to_string(),
                format!("{}://", parsed_uri.scheme.unwrap()),
            );
            let configs = vec![updated_config];
            ObjectStoreHandler::list_buckets(&configs[0]).await
        }
    }

    pub async fn list_buckets(
        config: &Config,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        let object_stores = object_stores_from_config(config.clone()).await?;
        Ok(Some(ListObjectsResult::Buckets(object_stores)))
    }

    async fn list_files_in_bucket(
        &self,
        parsed_uri: ParsedUri,
        config: Config,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<CallbackWrapper>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        let bucket_uri = if let Some(scheme) = &parsed_uri.scheme {
            format!("{}://{}", scheme, parsed_uri.bucket.as_ref().unwrap())
        } else {
            format!("localfs://{}", parsed_uri.bucket.as_ref().unwrap())
        };
        let object_store = ObjectStore::new(&bucket_uri, config).unwrap();

        if let Some(callback) = callback {
            object_store
                .list_files_with_callback(
                    parsed_uri.prefix.as_deref(),
                    recursive,
                    max_files,
                    filter,
                    callback,
                )
                .await?;
            Ok(None)
        } else {
            let file_objects = object_store
                .list_files(
                    parsed_uri.prefix.as_deref(),
                    recursive,
                    max_files,
                    filter,
                )
                .await?;
            Ok(Some(ListObjectsResult::FileObjects(file_objects)))
        }
    }
}

#[async_trait]
pub trait ObjectStoreBackend {
    fn new(config: Config) -> Result<Self, LakestreamError>
    where
        Self: Sized;

    async fn list_buckets(
        config: Config,
    ) -> Result<Vec<ObjectStore>, LakestreamError>;
}
