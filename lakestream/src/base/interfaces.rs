use std::collections::HashMap;

use std::pin::Pin;

use log::{error, info};
use regex::Regex;
use serde_json::{Map, Value};
use futures::Future;

pub use super::file_objects::{FileObject, FileObjectVec};
pub use super::object_store::{ObjectStore, ObjectStoreTrait};
use crate::s3::bucket::list_buckets;
use crate::s3::config::validate_config;
use crate::{Config, FileObjectFilter, LakestreamError};


pub enum CallbackWrapper {
    Sync(Box<dyn Fn(&[FileObject]) + Send + Sync + 'static>),
    // TODO: the Async version is not working properly yet
    Async(Box<dyn Fn(&[FileObject]) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static>),
}

pub enum ListObjectsResult {
    Buckets(Vec<ObjectStore>),
    FileObjects(Vec<FileObject>),
}

pub struct ObjectStoreHandler {
    configs: Vec<HashMap<String, Value>>,
}

impl ObjectStoreHandler {
    pub fn new(configs: Vec<HashMap<String, Value>>) -> Self {
        ObjectStoreHandler { configs }
    }

    pub async fn list_objects(
        uri: String,
        config: Config,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
    ) -> Result<ListObjectsResult, LakestreamError> {
        let (scheme, bucket, prefix) = ObjectStoreHandler::parse_uri(uri);
        if let Some(bucket) = bucket {
            // list files in a bucket
            info!("Listing files in bucket {}", bucket);
            let bucket_uri = if let Some(scheme) = scheme {
                format!("{}://{}", scheme, bucket)
            } else {
                format!("localfs://{}", bucket)
            };
            let object_store = ObjectStore::new(&bucket_uri, config).unwrap();

            let file_objects = object_store
                .list_files(prefix.as_deref(), recursive, max_files, filter)
                .await?;
            Ok(ListObjectsResult::FileObjects(file_objects))
        } else {
            // list buckets
            info!("Listing buckets");
            let mut object_store_configuration = HashMap::new();
            // Convert the HashMap<String, String> back to serde_json::Map<String, Value>
            let config_map: Map<String, Value> = config
                .settings
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect();
            object_store_configuration
                .insert("config".to_string(), Value::Object(config_map));
            object_store_configuration.insert(
                "uri".to_string(),
                Value::String(format!("{}://", scheme.unwrap())),
            );
            let configs = vec![object_store_configuration];
            let handler = ObjectStoreHandler::new(configs);

            let object_stores = handler.list_object_stores().await?;
            Ok(ListObjectsResult::Buckets(object_stores))
        }
    }

    pub async fn list_objects_with_callback(
        uri: String,
        config: Config,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: CallbackWrapper,
    ) -> Result<(), LakestreamError> {
        let (scheme, bucket, prefix) = ObjectStoreHandler::parse_uri(uri);
        if let Some(bucket) = bucket {
            // list files in a bucket
            info!("Listing files in bucket {}", bucket);
            let bucket_uri = if let Some(scheme) = scheme {
                format!("{}://{}", scheme, bucket)
            } else {
                format!("localfs://{}", bucket)
            };
            let object_store = ObjectStore::new(&bucket_uri, config).unwrap();

            object_store
                .list_files_with_callback(prefix.as_deref(), recursive, max_files, filter, callback)
                .await?;

            Ok(())
        } else {
            // list buckets
            panic!("Listing buckets not yet supported with callback");
        }
    }

    pub fn parse_uri(
        uri: String,
    ) -> (Option<String>, Option<String>, Option<String>) {
        if uri.is_empty() {
            return (None, None, None);
        }

        let re = Regex::new(r"^(?P<scheme>[a-z0-9]+)://").unwrap();
        let scheme_match = re.captures(&uri);

        scheme_match.map_or_else(
            || {
                // uri has no scheme, assume LocalFs
                let (bucket, prefix) = parse_uri_path(&uri);
                (None, bucket, prefix)
            },
            |scheme_captures| {
                let scheme = scheme_captures.name("scheme").unwrap().as_str();
                let uri_without_scheme = re.replace(&uri, "").to_string();
                if uri_without_scheme.is_empty() {
                    (Some(scheme.to_string()), None, None)
                } else {
                    let (bucket, prefix) = parse_uri_path(&uri_without_scheme);
                    (Some(scheme.to_string()), bucket, prefix)
                }
            },
        )
    }

    pub async fn list_object_stores(
        &self,
    ) -> Result<Vec<ObjectStore>, LakestreamError> {
        let mut object_stores = Vec::new();

        for config in &self.configs {
            let default_uri = Value::String("".to_string());
            let uri = config
                .get("uri")
                .unwrap_or(&default_uri)
                .as_str()
                .unwrap_or("");
            let config_value = config.get("config").unwrap();
            let config_config = config_value.as_object().unwrap();

            // Convert the serde_json::Map<String, Value> to HashMap<String, String>
            let config_hashmap: HashMap<String, String> = config_config
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                .collect();

            // Create a Config instance
            let mut config_instance = Config {
                settings: config_hashmap,
            };

            if let Err(e) = validate_config(&mut config_instance) {
                // Handle the error, e.g., log the error and/or return early with an appropriate error value
                error!("Error validating the config: {}", e);
                return Err(LakestreamError::ConfigError(
                    "Invalid configuration".to_string(),
                ));
            }

            if uri.starts_with("s3://") {
                match list_buckets(&config_instance).await {
                    Ok(mut buckets) => object_stores.append(&mut buckets),
                    Err(err) => error!("Error listing buckets: {}", err),
                }
            } else {
                error!("Unsupported object store type: {}", uri);
            }
        }
        Ok(object_stores)
    }
}

fn parse_uri_path(uri_path: &str) -> (Option<String>, Option<String>) {
    let cleaned_uri = uri_path.trim_end_matches('.');

    if cleaned_uri.is_empty() {
        return (Some(".".to_string()), None);
    }

    let is_absolute = cleaned_uri.starts_with('/');
    let mut parts = cleaned_uri.splitn(2, '/');
    let bucket = parts.next().map(|s| s.to_string());
    let prefix = parts.next().filter(|s| !s.is_empty()).map(|s| {
        let cleaned_prefix = s.replace("./", "");
        if cleaned_prefix.ends_with('/') {
            cleaned_prefix
        } else {
            format!("{}/", cleaned_prefix)
        }
    });

    if let Some(bucket) = bucket {
        let formatted_bucket = if is_absolute {
            format!("/{}", bucket)
        } else {
            bucket
        };
        return (Some(formatted_bucket), prefix);
    }

    (Some(".".to_string()), None)
}
