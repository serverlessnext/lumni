use base64::engine::general_purpose;
use base64::Engine as _;
use js_sys::Uint8Array;
use leptos::log;
use wasm_bindgen::JsValue;
use web_sys::{window, CryptoKey};

use super::crypto::{decrypt, encrypt, get_crypto_subtle};
use super::error::SecureStringError;
use super::FormOwner;

const KEY_PREFIX: &str = "STRINGVAULT";

type SecureStringResult<T> = Result<T, SecureStringError>;

pub fn create_storage_key(form_owner: &FormOwner) -> String {
    vec![KEY_PREFIX, &form_owner.tag, &form_owner.id].join(":")
}

pub async fn save_secure_string(
    form_owner: FormOwner,
    value: &str,
    crypto_key: &CryptoKey,
) -> SecureStringResult<()> {
    let (_, crypto, _) = get_crypto_subtle()?;

    let mut iv = [0u8; 12];
    crypto.get_random_values_with_u8_array(&mut iv)?;
    let (encrypted_data, iv_vec) = encrypt(crypto_key, value, &iv).await?;
    let encrypted_data = js_sys::Uint8Array::new(&encrypted_data);
    let encrypted_data: Vec<u8> = encrypted_data.to_vec();
    let encrypted_data_with_iv = [iv_vec, encrypted_data].concat();
    let encrypted_data_with_iv_base64 =
        general_purpose::STANDARD.encode(&encrypted_data_with_iv);

    let storage_key = create_storage_key(&form_owner);
    save_string(&storage_key, &encrypted_data_with_iv_base64).await?;
    Ok(())
}

pub async fn load_secure_string(
    form_owner: FormOwner,
    crypto_key: &CryptoKey,
) -> SecureStringResult<String> {
    let storage_key = create_storage_key(&form_owner);
    let encrypted_data_base64 = load_string(&storage_key)
        .await
        .ok_or(SecureStringError::NoLocalStorageData)?;

    let encrypted_data_with_iv =
        general_purpose::STANDARD.decode(&encrypted_data_base64)?;

    let (iv, encrypted_data) = encrypted_data_with_iv.split_at(12);
    let encrypted_data = Uint8Array::from(&encrypted_data[..]);
    let iv = Uint8Array::from(&iv[..]);

    let decrypted_data = decrypt(crypto_key, &encrypted_data, &iv).await?;
    Ok(decrypted_data)
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
