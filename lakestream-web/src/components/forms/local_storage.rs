use std::collections::HashMap;

use leptos::log;
use localencrypt::{LocalEncrypt, LocalStorage};

use crate::components::input::DisplayValue;
use crate::components::forms::{FormData, FormError};

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";

#[derive(Clone, Debug)]
pub struct ConfigurationFormMeta {
    pub form_id: String,
    pub config_name: String,
    pub template_name: String,
    pub tags: Option<HashMap<String, String>>, // original tags
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
            let config_name = tags
                .get("ConfigName")
                .cloned()
                .ok_or_else(|| "ConfigName not found".to_string())?;

            let template_name = tags
                .get("TemplateName")
                .cloned()
                .ok_or_else(|| "TemplateName not found".to_string())?;

            Ok(ConfigurationFormMeta {
                form_id: form_id.to_string(),
                config_name,
                template_name,
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
                    Ok(config) => Ok(Some(config)),
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
        let form_state = form_data.form_state().clone();
        let form_config: HashMap<String, String> = form_state
            .iter()
            .map(|(key, element_state)| {
                (
                    key.clone(),
                    match element_state.read_display_value() {
                        DisplayValue::Text(text) => text,
                        _ => unreachable!(),
                    },
                )
            })
            .collect();

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
        let meta_data = form_data.meta_data().clone();
        log!("Saving with form data: {:?}", meta_data);
        match local_storage
            .save_content(meta_data, &document_content)
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
