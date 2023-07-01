use std::collections::HashMap;

use leptos::log;
use localencrypt::LocalEncrypt;

use crate::components::form_input::DisplayValue;
use crate::components::forms::{FormData, FormError};

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";

const TEMPLATE_DEFAULT: &str = "Environment";

#[derive(Clone, Debug)]
pub struct ConfigurationFormMeta {
    pub form_id: String,
    pub config_name: String,
    pub template_name: String,
    pub tags: Option<HashMap<String, String>>, // original tags
}

pub async fn get_form_info_from_vault(
    vault: &LocalEncrypt,
    form_id: &str,
) -> Result<ConfigurationFormMeta, String> {
    let local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("{}", INVALID_BROWSER_STORAGE_TYPE))
        }
        _ => panic!("{}", INVALID_STORAGE_BACKEND),
    };

    let configurations =
        local_storage.list_items().await.unwrap_or_else(|_| vec![]);

    let form_data_option = configurations
        .iter()
        .find(|form_data| form_data.id() == form_id);

    match form_data_option {
        Some(form_data) => {

            let tags = form_data.tags();

            let config_name = form_data.tags().and_then(|tags| {
                tags.get("ConfigName")
                    .cloned()
                    .or_else(|| Some("Untitled".to_string()))
            });

            log!("FormData found: {:?}", form_data);
            // defaults to "Environment"
            let template_name = form_data
                .tags()
                .and_then(|tags| tags.get("TemplateName").cloned())
                .unwrap_or_else(|| TEMPLATE_DEFAULT.to_string());

            if let Some(config_name) = config_name {
                Ok(ConfigurationFormMeta {
                    form_id: form_id.to_string(),
                    config_name,
                    template_name,
                    tags,
                })
            } else {
                Err("Form name not found".to_string())
            }
        }
        None => Err("Form data not found".to_string()),
    }
}

pub async fn load_config_from_vault(
    vault: &LocalEncrypt,
    form_id: &str,
) -> Result<Option<HashMap<String, String>>, String> {
    let local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("{}", INVALID_BROWSER_STORAGE_TYPE))
        }
        _ => panic!("{}", INVALID_STORAGE_BACKEND),
    };

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

pub async fn save_config_to_vault(
    vault: &LocalEncrypt,
    form_data: &FormData,
) -> Result<(), FormError> {
    // convert form data to a HashMap<String, String>
    let form_state = form_data.form_state().clone();
    let form_config: HashMap<String, String> = form_state
        .iter()
        .map(|(key, element_state)| {
            (key.clone(), match element_state.read_display_value() {
                DisplayValue::Text(text) => text,
                _ => unreachable!(), // We've checked for Binary data above, so this should never happen
            })
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

    // Get the local storage from the vault
    let mut local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("{}", INVALID_BROWSER_STORAGE_TYPE))
        }
        _ => panic!("{}", INVALID_STORAGE_BACKEND),
    };

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
