use std::collections::HashMap;

use regex::Regex;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::s3::bucket::{list_buckets, S3Bucket};
use crate::utils::formatters::{bytes_human_readable, time_human_readable};

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

    pub fn list_objects(
        uri: String,
        config: HashMap<String, String>,
        max_files: Option<u32>,
    ) -> ListObjectsResult {
        let (scheme, bucket, prefix) = ObjectStoreHandler::parse_uri(uri);

        if let Some(bucket) = bucket {
            let bucket_uri = format!("{}://{}", scheme.unwrap(), bucket);
            let object_store = ObjectStore::new(&bucket_uri, config).unwrap();
            let file_objects =
                object_store.list_files(prefix.as_deref(), max_files);
            ListObjectsResult::FileObjects(file_objects)
        } else {
            let mut object_store_configuration = HashMap::new();
            let config_map: Map<String, Value> = config
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect();
            object_store_configuration
                .insert("config".to_string(), Value::Object(config_map));
            object_store_configuration.insert(
                "uri".to_string(),
                Value::String(format!("{}://", scheme.unwrap()).to_string()),
            );

            let configs = vec![object_store_configuration];
            let handler = ObjectStoreHandler::new(configs);

            let object_stores = handler.list_object_stores();
            ListObjectsResult::Buckets(object_stores)
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

        if let Some(scheme_captures) = scheme_match {
            let scheme = scheme_captures.name("scheme").unwrap().as_str();
            let uri_without_scheme = re.replace(&uri, "");
            if uri_without_scheme.is_empty() {
                return (Some(scheme.to_string()), None, None);
            }
            let mut parts = uri_without_scheme.splitn(2, '/');
            let bucket = parts.next().map(|s| s.to_string());
            let prefix = parts
                .next()
                .map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        if s.ends_with('/') {
                            Some(s.to_string())
                        } else {
                            Some(format!("{}/", s))
                        }
                    }
                })
                .flatten();

            (Some(scheme.to_string()), bucket, prefix)
        } else {
            eprintln!("Error: Invalid URI or missing scheme.");
            (None, None, None)
        }
    }

    pub fn list_object_stores(&self) -> Vec<ObjectStore> {
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

            // Convert the serde_json::Map<String, Value> back to HashMap<String, String>
            let config_hashmap: HashMap<String, String> = config_config
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                .collect();

            if uri.starts_with("s3://") {
                object_stores.append(&mut list_buckets(&config_hashmap));
            } else {
                eprintln!("Unsupported object store type: {}", uri);
            }
        }
        object_stores
    }
}

pub trait ObjectStoreTrait {
    fn name(&self) -> &str;
    fn list_files(
        &self,
        prefix: Option<&str>,
        max_keys: Option<u32>,
    ) -> Vec<FileObject>;
}

pub enum ObjectStore {
    S3Bucket(S3Bucket),
    // Add more object store types here later.
}

impl ObjectStore {
    pub fn new(
        name: &str,
        config: HashMap<String, String>,
    ) -> Result<ObjectStore, String> {
        if name.starts_with("s3://") {
            let name = name.trim_start_matches("s3://");
            let bucket = S3Bucket::new(name, config);
            Ok(ObjectStore::S3Bucket(bucket))
        } else {
            Err("Unsupported object store.".to_string())
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.name(),
        }
    }

    pub fn list_files(
        &self,
        prefix: Option<&str>,
        max_keys: Option<u32>,
    ) -> Vec<FileObject> {
        match self {
            ObjectStore::S3Bucket(bucket) => {
                bucket.list_files(prefix, max_keys)
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileObject {
    name: String,
    size: u64,
    modified: Option<u64>,
    tags: Option<HashMap<String, String>>,
}

impl FileObject {
    pub fn new(
        name: String,
        size: u64,
        modified: Option<u64>,
        tags: Option<HashMap<String, String>>,
    ) -> Self {
        FileObject {
            name,
            size,
            modified,
            tags,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn modified(&self) -> Option<u64> {
        self.modified
    }

    pub fn tags(&self) -> &Option<HashMap<String, String>> {
        &self.tags
    }

    pub fn printable(&self) -> String {
        let name_without_trailing_slash = self.name.trim_end_matches('/');
        let mut basename = name_without_trailing_slash
            .split('/')
            .last()
            .unwrap_or(name_without_trailing_slash)
            .to_string();

        if self.name.ends_with('/') {
            basename.push('/');
        }
        format!(
            "{:8} {} {}",
            bytes_human_readable(self.size()),
            if let Some(modified) = self.modified() {
                time_human_readable(modified)
            } else {
                "PRE".to_string()
            },
            basename
        )
    }
}
