use leptos::*;
use leptos::logging::log;
use wasm_bindgen::prelude::*;

use crate::components::forms::{
    FormStorage, FormStorageHandler, LocalStorageWrapper,
};
use crate::GlobalState;

pub fn create_storage_handler(
) -> Option<FormStorageHandler<LocalStorageWrapper>> {
    let vault_option = use_context::<RwSignal<GlobalState>>();

    match vault_option {
        Some(vault_signal) => {
            let vault_result =
                vault_signal.with_untracked(|state| state.vault.clone());

            match vault_result {
                Some(vault) => {
                    log!("Vault has been initialized");
                    let storage_wrapper = LocalStorageWrapper::new(vault);
                    Some(FormStorageHandler::new(storage_wrapper))
                }
                None => {
                    log::error!("Vault has not been initialized");
                    None
                }
            }
        }
        None => {
            log::error!("GlobalState has not been provided");
            None
        }
    }
}

pub fn local_storage_handler() -> Option<Box<dyn FormStorage>> {
    let vault_option = use_context::<RwSignal<GlobalState>>();

    match vault_option {
        Some(vault_signal) => {
            let vault_result =
                vault_signal.with_untracked(|state| state.vault.clone());

            match vault_result {
                Some(vault) => {
                    log!("Vault has been initialized");
                    // Directly return a boxed LocalStorageWrapper that implements FormStorage
                    Some(Box::new(LocalStorageWrapper::new(vault))
                        as Box<dyn FormStorage>)
                }
                None => {
                    log::error!("Vault has not been initialized");
                    None
                }
            }
        }
        None => {
            log::error!("GlobalState has not been provided");
            None
        }
    }
}

pub async fn list_all_keys() -> Result<Vec<String>, JsValue> {
    let mut result = Vec::new();

    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let len = storage.length().map_err(|_| {
                JsValue::from_str(
                    "Error: Unable to get the length of localStorage.",
                )
            })?;

            for i in 0..len {
                if let Ok(Some(key)) = storage.key(i) {
                    result.push(key);
                } else {
                    return Err(JsValue::from_str(
                        "Error: Unable to get a key from localStorage.",
                    ));
                }
            }

            Ok(result)
        } else {
            Err(JsValue::from_str("Error: localStorage is not available."))
        }
    } else {
        Err(JsValue::from_str("Error: Unable to access window object."))
    }
}

pub async fn delete_keys_not_matching_prefix(
    prefix: &str,
) -> Result<(), JsValue> {
    let keys = list_all_keys().await?;

    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            // Delete each key in the list that does not match the prefix
            for key in keys {
                if !key.starts_with(prefix) {
                    storage.remove_item(&key).map_err(|_| {
                        JsValue::from_str(
                            "Error: Unable to remove data from localStorage.",
                        )
                    })?;
                }
            }

            Ok(())
        } else {
            Err(JsValue::from_str("Error: localStorage is not available."))
        }
    } else {
        Err(JsValue::from_str("Error: Unable to access window object."))
    }
}
