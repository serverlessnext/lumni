use web_sys::window;
use serde::{Deserialize, Serialize};


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
