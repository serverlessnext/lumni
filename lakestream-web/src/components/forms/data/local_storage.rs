use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use localencrypt::{ItemMetaData, LocalEncrypt, LocalStorage};

use super::form_storage::{ConfigurationFormMeta, FormStorage};

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";

#[derive(Clone, Debug)]
pub struct LocalStorageWrapper {
    vault: LocalEncrypt,
}

impl LocalStorageWrapper {
    pub fn new(vault: LocalEncrypt) -> Self {
        LocalStorageWrapper { vault }
    }

    fn get_local_storage(&self) -> Result<LocalStorage, String> {
        match self.vault.backend() {
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
        Box::pin(async {
            let local_storage = self.get_local_storage()?;
            let items = local_storage.list_items().await.unwrap_or_default();
            let configurations: Vec<ConfigurationFormMeta> = items
                .into_iter()
                .map(|item| {
                    let tags = item.tags().unwrap_or_default();
                    let profile_name = tags.get("ProfileName").cloned();
                    let app_name = tags.get("AppName").cloned();

                    ConfigurationFormMeta::new(
                        item.id(),
                        profile_name.unwrap_or_default(),
                        app_name.unwrap_or_default(),
                    )
                    .with_tags(tags)
                })
                .collect();
            Ok(configurations)
        })
    }

    fn load_content<'a>(
        &'a self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, String>> + '_>>
    {
        Box::pin(async {
            let local_storage = self.get_local_storage()?;
            let content_result = local_storage.load_content(id).await;
            match content_result {
                Ok(Some(data)) => {
                    let config: HashMap<String, String> =
                        serde_json::from_slice(&data)
                            .map_err(|e| e.to_string())?;
                    Ok(Some(
                        serde_json::to_vec(&config)
                            .map_err(|e| e.to_string())?,
                    ))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn save_content<'a>(
        &'a self,
        form_meta: &ConfigurationFormMeta,
        content: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
        let tags = form_meta.tags();
        let item_meta_data = ItemMetaData::new_with_tags(
            &form_meta.id(),
            tags.unwrap_or_default(),
        );
        Box::pin(async {
            let form_config: HashMap<String, String> =
                serde_json::from_slice(content).map_err(|e| e.to_string())?;
            let document_content =
                serde_json::to_vec(&form_config).map_err(|e| e.to_string())?;
            let mut local_storage = self.get_local_storage()?;
            local_storage
                .save_content(item_meta_data, &document_content)
                .await
                .map_err(|e| e.to_string())
        })
    }
}
