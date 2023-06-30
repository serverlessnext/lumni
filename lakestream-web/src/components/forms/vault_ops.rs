use std::collections::HashMap;

use leptos::log;
use localencrypt::LocalEncrypt;

use crate::components::form_input::DisplayValue;
use crate::components::forms::{FormData, FormError};

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";

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
