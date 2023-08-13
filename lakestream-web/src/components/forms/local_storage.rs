use std::collections::HashMap;

use leptos::log;
use localencrypt::{LocalEncrypt, LocalStorage, ItemMetaData};

use crate::components::forms::{FormData, FormError};

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";

#[derive(Clone, Debug)]
pub struct ConfigurationFormMeta {
    id: String,
    name: Option<String>,
    template: Option<String>,
    tags: Option<HashMap<String, String>>, // original tags
}

#[allow(dead_code)]
impl ConfigurationFormMeta {
    pub fn new<S: Into<String>>(id: S, name: S, template: S) -> Self {
        Self {
            id: id.into(),
            name: Some(name.into()),
            template: Some(template.into()),
            tags: None,
        }
    }

    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_id<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            name: None,
            template: None,
            tags: None,
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn template(&self) -> Option<String> {
        self.template.clone()
    }

    pub fn tags(&self) -> Option<HashMap<String, String>> {
        self.tags.clone()
    }
}

#[derive(Clone, Debug)]
pub struct FormStorageHandler {
    vault: LocalEncrypt,
}

impl FormStorageHandler {
    pub fn new(vault: LocalEncrypt) -> Self {
        FormStorageHandler { vault }
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

    pub async fn get_form_info(
        &self,
        form_id: &str,
    ) -> Result<Option<HashMap<String, String>>, String> {
        let local_storage = self.get_local_storage()?;

        let configurations =
            local_storage.list_items().await.unwrap_or_default();

        let form_data_option = configurations
            .iter()
            .find(|form_data| form_data.id() == form_id);

        match form_data_option {
            Some(form_data) => Ok(form_data.tags()),
            None => Err("Form data not found".to_string()),
        }
    }

    pub async fn get_configuration_meta(
        &self,
        form_id: &str,
    ) -> Result<ConfigurationFormMeta, String> {
        let tags_opt = self.get_form_info(form_id).await?;
        if let Some(tags) = tags_opt {
            let profile_name = tags.get("ConfigName").cloned();
            let template_name = tags.get("TemplateName").cloned();

            Ok(ConfigurationFormMeta {
                id: form_id.to_string(),
                name: profile_name,
                template: template_name,
                tags: Some(tags),
            })
        } else {
            Err("Form data not found".to_string())
        }
    }

    pub async fn load_config(
        &self,
        form_id: &str,
    ) -> Result<Option<HashMap<String, String>>, String> {
        let local_storage = self.get_local_storage()?;
        let content_result = local_storage.load_content(form_id).await;

        match content_result {
            Ok(Some(data)) => {
                match serde_json::from_slice::<HashMap<String, String>>(&data) {
                    Ok(config) => {
                        Ok(Some(config))
                    }
                    Err(e) => {
                        log::error!("error deserializing config: {:?}", e);
                        Err(e.to_string())
                    }
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn save_config(
        &self,
        form_data: &FormData,
    ) -> Result<(), FormError> {
        let form_config = form_data.export_config();

        // Serialize form data into JSON
        let document_content = match serde_json::to_vec(&form_config) {
            Ok(content) => content,
            Err(e) => {
                log::error!("error serializing config: {:?}", e);
                return Err(FormError::SubmitError(e.to_string()));
            }
        };

        let mut local_storage = self.get_local_storage()?;

        // Save the serialized form data to the local storage
        let tags = form_data.meta_data().tags.clone();
        let item_meta_data = ItemMetaData::new_with_tags(
            &form_data.meta_data().id,
            tags.unwrap_or_default(),
        );
        match local_storage
            .save_content(item_meta_data, &document_content)
            .await
        {
            Ok(_) => {
                log!("Successfully saved form data");
                Ok(())
            }
            Err(e) => {
                log!("Failed to save form data. Error: {:?}", e);
                Err(FormError::SubmitError(e.to_string()))
            }
        }
    }
}
