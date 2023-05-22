use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::stringvault::config_handler::ConfigManager;
use crate::utils::local_storage::{load_from_storage, save_to_storage};

const LOCAL_STORAGE_KEY: &str = "OBJECT_STORES";

#[derive(Debug, Clone)]
pub struct ObjectStoreList {
    pub items: Vec<ObjectStore>,
}

impl ObjectStoreList {
    pub fn new() -> Self {
        let initial_items = Self::load_from_local_storage();
        Self {
            items: initial_items,
        }
    }

    pub fn load_from_local_storage() -> Vec<ObjectStore> {
        load_from_storage::<Vec<ItemSerialized>>(LOCAL_STORAGE_KEY)
            .map(|values| {
                values
                    .into_iter()
                    .map(|stored| stored.into_item())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn save_to_local_storage(&self) {
        save_to_storage(
            LOCAL_STORAGE_KEY,
            &self
                .items
                .iter()
                .map(ItemSerialized::from)
                .collect::<Vec<_>>(),
        );
    }

    // Add and remove now operate on non-reactive types
    pub fn add(&mut self, item: ObjectStore) {
        self.items.push(item);
    }

    pub fn remove(&mut self, id: Uuid) {
        self.items.retain(|item| item.id != id);
    }
}

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

impl ItemSerialized {
    pub fn into_item(self) -> ObjectStore {
        ObjectStore {
            id: self.id,
            uri: self.uri,
        }
    }
}

impl From<&ObjectStore> for ItemSerialized {
    fn from(item: &ObjectStore) -> Self {
        Self {
            id: item.id,
            uri: item.uri.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ItemSerialized {
    pub id: Uuid,
    pub uri: String,
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
