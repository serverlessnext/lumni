use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::window;

pub async fn save_data(key: &str, value: &str) -> Result<(), JsValue> {
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

pub async fn load_data(key: &str) -> Option<String> {
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

pub fn load_from_storage<T: for<'de> Deserialize<'de>>(key: &str) -> Option<T> {
    if let Ok(Some(storage)) = window()?.local_storage() {
        storage
            .get_item(key)
            .ok()
            .flatten()
            .and_then(|value| serde_json::from_str::<T>(&value).ok())
    } else {
        None
    }
}

pub fn save_to_storage<T: Serialize>(key: &str, value: &T) {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let json =
                serde_json::to_string(value).expect("Couldn't serialize value");
            if storage.set_item(key, &json).is_err() {
                log::error!("Error while trying to set item in localStorage");
            }
        }
    }
}
