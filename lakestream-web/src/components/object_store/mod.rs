use std::collections::HashMap;
use async_trait::async_trait;
use uuid::Uuid;

use crate::components::stringvault::config_handler::ConfigManager;

mod list;
pub use list::{ObjectStoreList, ObjectStoreListView};


#[derive(Debug, Clone)]
pub struct ObjectStore {
    pub id: Uuid,
    pub uri: String,
}

impl ObjectStore {
    pub fn new(id: Uuid, uri: String) -> Self {
        Self { id, uri }
    }

    pub fn get_default_config(&self) -> HashMap<String, String> {
        // TODO: This should be a match on the URI scheme
        s3_default_config()
    }
}

#[async_trait(?Send)]
impl ConfigManager for ObjectStore {
    fn get_default_config(&self) -> HashMap<String, String> {
        ObjectStore::get_default_config(self)
    }
    fn id(&self) -> String {
        self.id.to_string()
    }
}



fn s3_default_config() -> HashMap<String, String> {
    let mut config = HashMap::new();
    config.insert("BUCKET_URI".to_string(), "s3://".to_string());
    config.insert("AWS_ACCESS_KEY_ID".to_string(), "".to_string());
    config.insert("AWS_SECRET_ACCESS_KEY".to_string(), "".to_string());
    config.insert("AWS_REGION".to_string(), "auto".to_string());
    config.insert("S3_ENDPOINT_URL".to_string(), "".to_string());
    config
}
