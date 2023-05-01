use std::collections::HashMap;

use log::{error, info};
use regex::Regex;
use serde_json::{Map, Value};

pub use super::callback_wrapper::CallbackWrapper;
pub use super::file_objects::{FileObject, FileObjectVec};
pub use super::object_store::{ObjectStore, ObjectStoreTrait};
use crate::s3::bucket::list_buckets;
use crate::s3::config::validate_config;
use crate::{Config, FileObjectFilter, LakestreamError};

pub enum ListObjectsResult {
    Buckets(Vec<ObjectStore>),
    FileObjects(Vec<FileObject>),
}

pub struct ObjectStoreHandler {
    configs: Option<Vec<Config>>,
}

struct ParsedUri {
    pub scheme: Option<String>,
    pub bucket: Option<String>,
    pub prefix: Option<String>,
}

impl ObjectStoreHandler {
    pub fn new(configs: Option<Vec<Config>>) -> Self {
        ObjectStoreHandler { configs }
    }

    pub async fn list_buckets(
        &self,
        scheme: Option<String>,
        mut config: Config,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        // Update the config
        config
            .settings
            .insert("uri".to_string(), format!("{}://", scheme.unwrap()));

        // Create a new ObjectStoreHandler with a Vec<Config> containing the updated config
        let configs = vec![config];
        let handler = ObjectStoreHandler::new(Some(configs));

        let object_stores = handler.list_object_stores().await?;
        Ok(Some(ListObjectsResult::Buckets(object_stores)))
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
        let parsed_uri = ObjectStoreHandler::parse_uri(uri);
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
            self.list_buckets(parsed_uri.scheme, config).await
        }
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

    fn parse_uri(uri: String) -> ParsedUri {
        if uri.is_empty() {
            return ParsedUri {
                scheme: None,
                bucket: None,
                prefix: None,
            };
        }

        let re = Regex::new(r"^(?P<scheme>[a-z0-9]+)://").unwrap();
        let scheme_match = re.captures(&uri);

        scheme_match.map_or_else(
            || {
                // uri has no scheme, assume LocalFs
                let (bucket, prefix) = parse_uri_path(&uri);
                ParsedUri {
                    scheme: None,
                    bucket,
                    prefix,
                }
            },
            |scheme_captures| {
                let scheme = scheme_captures.name("scheme").unwrap().as_str();
                let uri_without_scheme = re.replace(&uri, "").to_string();
                if uri_without_scheme.is_empty() {
                    ParsedUri {
                        scheme: Some(scheme.to_string()),
                        bucket: None,
                        prefix: None,
                    }
                } else {
                    let (bucket, prefix) = parse_uri_path(&uri_without_scheme);
                    ParsedUri {
                        scheme: Some(scheme.to_string()),
                        bucket,
                        prefix,
                    }
                }
            },
        )
    }

    pub async fn list_object_stores(
        &self,
    ) -> Result<Vec<ObjectStore>, LakestreamError> {
        let mut object_stores = Vec::new();

        if let Some(configs) = &self.configs {
            for config in configs {
                let default_uri = Value::String("".to_string());
                let config_value = config
                    .settings
                    .get("uri")
                    .map(|v| Value::String(v.clone()))
                    .unwrap_or(default_uri);
                let uri = config_value.as_str().unwrap_or("");

                let config_config_value = config
                    .settings
                    .get("config")
                    .map(|v| Value::String(v.clone()))
                    .unwrap_or(Value::Object(Map::new()));
                let config_config = config_config_value
                    .as_object()
                    .expect("config_value should be an object")
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                    .collect::<HashMap<String, String>>();

                // Create a mutable Config instance
                let mut config_instance = Config {
                    settings: config_config,
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
