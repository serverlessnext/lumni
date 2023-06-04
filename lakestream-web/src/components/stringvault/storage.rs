use leptos::log;
use wasm_bindgen::JsValue;
use web_sys::window;

use super::object_key::ObjectKey;

const KEY_PREFIX: &str = "STRINGVAULT";

pub fn create_storage_key(object_key: &ObjectKey) -> String {
    vec![KEY_PREFIX, &object_key.tag(), &object_key.id()].join(":")
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

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_save_and_load_string() {
        let key = "test_key";
        let value = "test_value";
        let save_result = save_string(key, value).await;
        assert!(save_result.is_ok());
        let loaded_value = load_string(key).await;
        assert_eq!(loaded_value, Some(value.to_string()));
    }

    #[wasm_bindgen_test]
    async fn test_delete_string() {
        let key = "test_key";
        let value = "test_value";
        let save_result = save_string(key, value).await;
        assert!(save_result.is_ok());
        let delete_result = delete_string(key).await;
        assert!(delete_result.is_ok());
        let loaded_value = load_string(key).await;
        assert_eq!(loaded_value, None);
    }

    #[wasm_bindgen_test]
    fn test_create_storage_key() {
        let object_key = ObjectKey::new("tag", "id").unwrap();
        let storage_key = create_storage_key(&object_key);
        assert_eq!(storage_key, "STRINGVAULT:tag:id");
    }
}
