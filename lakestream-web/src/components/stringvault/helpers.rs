use base64::engine::general_purpose;
use base64::Engine as _;
use js_sys::{ArrayBuffer, Uint8Array};
use leptos::log;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, AesGcmParams, AesKeyGenParams, CryptoKey, Pbkdf2Params};

use super::error::SecureStringError;
use crate::utils::convert_types::{string_to_uint8array, uint8array_to_string};

type SecureStringResult<T> = Result<T, SecureStringError>;

fn get_crypto_subtle(
) -> SecureStringResult<(web_sys::Window, web_sys::Crypto, web_sys::SubtleCrypto)>
{
    let window = web_sys::window().ok_or(SecureStringError::NoWindow)?;
    let crypto = window.crypto().map_err(|_| SecureStringError::NoCrypto)?;
    let subtle = crypto.subtle();
    Ok((window, crypto, subtle))
}

async fn encrypt(
    key: &CryptoKey,
    data: &str,
    iv: &[u8],
) -> SecureStringResult<(ArrayBuffer, Vec<u8>)> {
    let (_, _, subtle) = get_crypto_subtle()?;
    let data = string_to_uint8array(data);
    let iv = Uint8Array::from(iv);
    let encrypted_data_promise = subtle.encrypt_with_object_and_buffer_source(
        &AesGcmParams::new("AES-GCM", &iv),
        key,
        &data,
    )?;
    let encrypted_data: js_sys::ArrayBuffer =
        wasm_bindgen_futures::JsFuture::from(encrypted_data_promise)
            .await?
            .dyn_into()?;
    Ok((encrypted_data, iv.to_vec()))
}

async fn decrypt(
    key: &CryptoKey,
    data: &Uint8Array,
    iv: &Uint8Array,
) -> SecureStringResult<String> {
    let (_, _, subtle) = get_crypto_subtle()?;
    let decrypted_data_promise = subtle.decrypt_with_object_and_buffer_source(
        &AesGcmParams::new("AES-GCM", iv),
        key,
        data,
    )?;

    let decrypted_data_result: js_sys::ArrayBuffer =
        wasm_bindgen_futures::JsFuture::from(decrypted_data_promise)
            .await?
            .dyn_into()?;

    let decrypted_data = js_sys::Uint8Array::new(&decrypted_data_result);
    let decrypted_data_vec: Vec<u8> = decrypted_data.to_vec();
    let decrypted_data = uint8array_to_string(&decrypted_data_vec);

    Ok(decrypted_data)
}

pub async fn save_secure_string(
    key: &str,
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

    save_string(key, &encrypted_data_with_iv_base64).await?;
    Ok(())
}

pub async fn load_secure_string(
    key: &str,
    crypto_key: &CryptoKey,
) -> SecureStringResult<String> {
    let encrypted_data_base64 = load_string(key)
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

pub async fn derive_crypto_key(
    password: &str,
    salt: &str,
) -> Result<CryptoKey, JsValue> {
    let iterations = 100000;
    let key_length = 256;
    let window = web_sys::window().expect("no global `window` exists");
    let crypto = window.crypto().expect("no `crypto` on `window`");
    let subtle = crypto.subtle();

    let password_data = string_to_uint8array(password);
    let salt_data = string_to_uint8array(salt);

    let key_usages_js = js_sys::Array::new();
    key_usages_js.push(&JsValue::from_str("deriveKey"));

    let password_key_promise = subtle.import_key_with_str(
        "raw",
        &password_data,
        "PBKDF2",
        false,
        &key_usages_js.into(),
    )?;

    let password_key: CryptoKey =
        wasm_bindgen_futures::JsFuture::from(password_key_promise)
            .await?
            .dyn_into()?;

    let key_usages_js = js_sys::Array::new();
    key_usages_js.push(&JsValue::from_str("encrypt"));
    key_usages_js.push(&JsValue::from_str("decrypt"));

    let derived_key_promise = subtle.derive_key_with_object_and_object(
        &Pbkdf2Params::new(
            "PBKDF2",
            &JsValue::from_str("SHA-256"),
            iterations,
            &salt_data.into(),
        ),
        &password_key,
        &AesKeyGenParams::new("AES-GCM", key_length),
        true,
        &key_usages_js.into(),
    )?;

    let derived_key: CryptoKey =
        wasm_bindgen_futures::JsFuture::from(derived_key_promise)
            .await?
            .dyn_into()?;

    Ok(derived_key)
}

pub fn generate_salt() -> Result<String, JsValue> {
    let salt_length = 16; // Your desired salt length
    let mut salt = vec![0u8; salt_length];

    web_sys::window()
        .expect("no global `window` exists")
        .crypto()
        .expect("should have a `Crypto` on the `Window`")
        .get_random_values_with_u8_array(&mut salt)
        .expect("get_random_values_with_u8_array failed");

    // Convert the salt array to a base64 string
    Ok(general_purpose::STANDARD.encode(&salt))
}

pub async fn get_or_generate_salt(user: &str) -> Result<String, JsValue> {
    let key = format!("USERS_{}", general_purpose::STANDARD.encode(user));
    match load_string(&key).await {
        Some(salt) => Ok(salt),
        None => {
            let new_salt = generate_salt()?;
            save_string(&key, &new_salt).await?;
            Ok(new_salt)
        }
    }
}

async fn save_string(key: &str, value: &str) -> Result<(), JsValue> {
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

async fn load_string(key: &str) -> Option<String> {
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
