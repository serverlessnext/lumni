use wasm_bindgen::prelude::*;

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
            Err(JsValue::from_str(
                "Error: localStorage is not available.",
            ))
        }
    } else {
        Err(JsValue::from_str(
            "Error: Unable to access window object.",
        ))
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
