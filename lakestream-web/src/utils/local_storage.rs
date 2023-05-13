
use leptos::log;
use web_sys::window;


pub fn save_data(key: &str, value: &str) {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Err(_) = storage.set_item(key, value) {
                log!("Error: Unable to save data to localStorage.");
            }
        } else {
            log!("Error: localStorage is not available.");
        }
    } else {
        log!("Error: Unable to access window object.");
    }
}

pub fn load_data(key: &str) -> Option<String> {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            return storage.get_item(key).unwrap_or(None);
        } else {
            log!("Error: localStorage is not available.");
        }
    } else {
        log!("Error: Unable to access window object.");
    }
    None
}
