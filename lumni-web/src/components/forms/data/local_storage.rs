use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use localencrypt::{ItemMetaData, LocalEncrypt, LocalStorage};

use super::form_storage::{ConfigurationFormMeta, FormStorage};

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";

#[derive(Clone, Debug)]
pub struct LocalStorageWrapper {
    vault: Arc<Mutex<LocalEncrypt>>,
}

impl LocalStorageWrapper {
    pub fn new(vault: LocalEncrypt) -> Self {
        LocalStorageWrapper {
            vault: Arc::new(Mutex::new(vault)),
        }
    }

    fn get_local_storage(&self) -> Result<LocalStorage, String> {
        let vault_guard = self.vault.lock().map_err(|e| e.to_string())?;
        match vault_guard.backend() {
            localencrypt::StorageBackend::Browser(browser_storage) => {
                Ok(browser_storage.local_storage().unwrap_or_else(|| {
                    panic!("{}", INVALID_BROWSER_STORAGE_TYPE)
                }))
            }
            _ => Err(INVALID_STORAGE_BACKEND.to_string()),
        }
    }
}

impl FormStorage for LocalStorageWrapper {
    fn list_items(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<ConfigurationFormMeta>, String>>
                + '_,
        >,
    > {
        Box::pin(async move {
            let local_storage = self.get_local_storage()?;
            let items = local_storage.list_items().await.unwrap_or_default();
            let configurations = items
                .into_iter()
                .map(|item| {
                    let tags = item.tags().unwrap_or_default();
                    ConfigurationFormMeta::new(
                        item.id(),
                        tags.get("ProfileName").cloned().unwrap_or_default(),
                        tags.get("AppName").cloned().unwrap_or_default(),
                    )
                    .with_tags(tags)
                })
                .collect();
            Ok(configurations)
        })
    }

    fn add_item(
        &self,
        item_meta: &ConfigurationFormMeta,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
        let tags = item_meta.tags().clone().unwrap_or_default();
        let item_meta_data = ItemMetaData::new_with_tags(&item_meta.id(), tags);

        Box::pin(async move {
            let mut local_storage = self.get_local_storage()?;
            local_storage
                .add_item(item_meta_data)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn delete_item(
        &self,
        item_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
        let item_id_owned = item_id.to_owned(); // Clone to break the lifetime dependency

        Box::pin(async move {
            let mut local_storage = self.get_local_storage()?;
            local_storage
                .delete_item(&item_id_owned)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn load_content(
        &self,
        id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, String>> + '_>>
    {
        let id_owned = id.to_owned(); // Clone to break the lifetime dependency

        Box::pin(async move {
            let local_storage = self.get_local_storage()?;
            local_storage
                .load_content(&id_owned)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn save_content(
        &self,
        form_meta: &ConfigurationFormMeta,
        content: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
        let id = form_meta.id();
        let tags = form_meta.tags().clone().unwrap_or_default();
        let item_meta_data = ItemMetaData::new_with_tags(&id, tags);
        let content_owned = content.to_vec();

        Box::pin(async move {
            // Deserialize content into an owned HashMap
            let form_config: HashMap<String, String> =
                serde_json::from_slice(&content_owned)
                    .map_err(|e| e.to_string())?;

            // Serialize the form_config into an owned Vec<u8>
            let document_content =
                serde_json::to_vec(&form_config).map_err(|e| e.to_string())?;

            let mut local_storage = self.get_local_storage()?;
            local_storage
                .save_content(item_meta_data, &document_content)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn clone_box(&self) -> Box<dyn FormStorage> {
        Box::new(self.clone())
    }
}
