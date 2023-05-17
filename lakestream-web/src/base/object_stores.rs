use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::local_storage::{load_from_storage, save_to_storage};

const LOCAL_STORAGE_KEY: &str = "OBJECT_STORES";

#[derive(Debug, Clone)]
pub struct ObjectStore {
    pub id: Uuid,
    pub uri: String,
}

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

impl ObjectStore {
    pub fn new(id: Uuid, uri: String) -> Self {
        Self { id, uri }
    }
}

impl ItemSerialized {
    pub fn into_item(self) -> ObjectStore {
        ObjectStore::new(self.id, self.uri)
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
