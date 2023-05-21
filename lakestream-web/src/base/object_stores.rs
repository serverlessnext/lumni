use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::stringvault::{SecureStringError, SecureStringResult};
use crate::StringVault;
use crate::utils::local_storage::{load_from_storage, save_to_storage};

const LOCAL_STORAGE_KEY: &str = "OBJECT_STORES";

#[derive(Debug, Clone)]
pub struct ObjectStore {
    pub id: Uuid,
    pub uri: String,
    pub vault: Arc<Mutex<StringVault>>,
}

#[derive(Debug, Clone)]
pub struct ObjectStoreList {
    pub items: Vec<ObjectStore>,
}

impl ObjectStoreList {
    pub fn new(vault: Arc<Mutex<StringVault>>) -> Self {
        let initial_items = Self::load_from_local_storage(vault);
        Self {
            items: initial_items,
        }
    }

    pub fn load_from_local_storage(vault: Arc<Mutex<StringVault>>) -> Vec<ObjectStore> {
        load_from_storage::<Vec<ItemSerialized>>(LOCAL_STORAGE_KEY)
            .map(|values| {
                values
                    .into_iter()
                    .map(|stored| stored.into_item(vault.clone()))
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

impl ObjectStore {
    pub fn new(id: Uuid, uri: String, vault: Arc<Mutex<StringVault>>) -> Self {
        Self { id, uri, vault }
    }

    pub fn get_default_config(&self) -> HashMap<String, String> {
        // TODO: This should be a match on the URI scheme
        s3_default_config()
    }

    pub async fn load_secure_configuration(&self) -> SecureStringResult<HashMap<String, String>> {
        let vault = self.vault.lock().unwrap();
        vault.load_secure_configuration(&self.id.urn().to_string()).await
    }

    pub async fn save_secure_configuration(
        &mut self,
        config: HashMap<String, String>,
    ) -> Result<(), SecureStringError> {
        let mut vault = self.vault.lock().unwrap();
        let uuid = self.id.urn().to_string();
        let result = vault.save_secure_configuration(&uuid, config.clone()).await;
        result
    }

}


impl ItemSerialized {
    pub fn into_item(self, vault: Arc<Mutex<StringVault>>) -> ObjectStore {
        ObjectStore {
            id: self.id,
            uri: self.uri,
            vault,
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
