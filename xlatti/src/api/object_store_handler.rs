use async_trait::async_trait;
use log::info;

use crate::base::object_store::object_stores_from_config;
use crate::utils::uri_parse::ParsedUri;
use crate::{
    BinaryCallbackWrapper, CallbackWrapper, EnvironmentConfig, FileObject,
    FileObjectFilter, LakestreamError, ListObjectsResult, ObjectStore,
    ObjectStoreVec,
};

#[derive(Clone)]
pub struct ObjectStoreHandler {}

impl ObjectStoreHandler {
    pub fn new(_configs: Option<Vec<EnvironmentConfig>>) -> Self {
        // creating with config will be used in future
        ObjectStoreHandler {}
    }

    pub async fn list_objects(
        &self,
        uri: &str,
        config: &EnvironmentConfig,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<CallbackWrapper<FileObject>>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(uri, true);

        if let Some(bucket) = &parsed_uri.bucket {
            // list files in a bucket
            info!("Listing files in bucket {}", bucket);
            self.list_files_in_bucket(
                parsed_uri,
                config.clone(),
                recursive,
                max_files,
                filter,
                callback,
            )
            .await
        } else {
            Err(LakestreamError::NoBucketInUri(uri.to_string()))
        }
    }

    pub async fn list_buckets(
        &self,
        uri: &str,
        config: &EnvironmentConfig,
        callback: Option<CallbackWrapper<ObjectStore>>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(uri, true);

        if let Some(bucket) = &parsed_uri.bucket {
            panic!("list_buckets called with bucket {}", bucket);
        } else {
            // list buckets
            // Clone the original config and update the settings
            // will change the input config to reference at future update
            let mut updated_config = config.clone();
            updated_config.settings.insert(
                "uri".to_string(),
                format!("{}://", parsed_uri.scheme.unwrap()),
            );

            let object_stores =
                object_stores_from_config(updated_config, &callback).await?;

            if callback.is_some() {
                // callback used, so can just return None
                Ok(None)
            } else {
                // no callback used, so convert the ObjectStoreVec to a Vec<ObjectStore>
                Ok(Some(ListObjectsResult::Buckets(object_stores.into_inner())))
            }
        }
    }

    pub async fn get_object(
        &self,
        uri: &str,
        config: &EnvironmentConfig,
        callback: Option<BinaryCallbackWrapper>,
    ) -> Result<Option<Vec<u8>>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(uri, false);

        if let Some(bucket) = &parsed_uri.bucket {
            // Get the object from the bucket
            let bucket_uri = if let Some(scheme) = &parsed_uri.scheme {
                format!("{}://{}", scheme, bucket)
            } else {
                format!("localfs://{}", bucket)
            };

            let key = parsed_uri.path.as_deref().unwrap();
            let object_store =
                ObjectStore::new(&bucket_uri, config.clone()).unwrap();

            // NOTE: initial callback implementation for get_object. In future updates the callback
            // mechanism will be pushed to the underlying object store methods, so we can add
            // chunking as well for increased performance and ability to handle big files that not
            // fit in memory
            if let Some(callback) = callback {
                let mut data = Vec::new();
                object_store.get_object(key, &mut data).await?;
                callback.call(data).await?;
                Ok(None)
            } else {
                let mut data = Vec::new();
                object_store.get_object(key, &mut data).await?;
                Ok(Some(data))
            }
        } else {
            Err(LakestreamError::NoBucketInUri(uri.to_string()))
        }
    }

    async fn list_files_in_bucket(
        &self,
        parsed_uri: ParsedUri,
        config: EnvironmentConfig,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<CallbackWrapper<FileObject>>,
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
                    parsed_uri.path.as_deref(),
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
                    parsed_uri.path.as_deref(),
                    recursive,
                    max_files,
                    filter,
                )
                .await?;
            Ok(Some(ListObjectsResult::FileObjects(file_objects)))
        }
    }
}

#[async_trait(?Send)]
pub trait ObjectStoreBackend: Send {
    fn new(config: EnvironmentConfig) -> Result<Self, LakestreamError>
    where
        Self: Sized;

    async fn list_buckets(
        config: EnvironmentConfig,
        object_stores: &mut ObjectStoreVec,
    ) -> Result<(), LakestreamError>;
}
