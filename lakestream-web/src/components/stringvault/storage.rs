use leptos::log;
use wasm_bindgen::JsValue;
use web_sys::window;

use super::FormOwner;

const KEY_PREFIX: &str = "STRINGVAULT";

pub fn create_storage_key(form_owner: &FormOwner) -> String {
    vec![KEY_PREFIX, &form_owner.tag, &form_owner.id].join(":")
}

pub async fn save_string(key: &str, value: &str) -> Result<(), JsValue> {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            storage.set_item(key, value).map_err(|_| {
                JsValue::from_str("Error: Unable to save data to localStorage.")
            })?;
            return Ok(());
        } else {
            return Err(JsValue::from_str(
                "Error: localStorage is not available.",
            ));
        }
    } else {
        return Err(JsValue::from_str(
            "Error: Unable to access window object.",
        ));
    }
}

pub async fn load_string(key: &str) -> Option<String> {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(data)) = storage.get_item(key) {
                return Some(data);
            }
        } else {
            log!("Error: localStorage is not available.");
        }
    } else {
        log!("Error: Unable to access window object.");
    }
    None
}

pub async fn delete_string(key: &str) -> Result<(), JsValue> {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            storage.remove_item(key).map_err(|_| {
                JsValue::from_str(
                    "Error: Unable to remove data from localStorage.",
                )
            })?;
            return Ok(());
        } else {
            return Err(JsValue::from_str(
                "Error: localStorage is not available.",
            ));
        }
    } else {
        return Err(JsValue::from_str(
            "Error: Unable to access window object.",
        ));
    }
}
