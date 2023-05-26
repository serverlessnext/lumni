use blake3::hash;
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{AesGcmParams, AesKeyGenParams, CryptoKey, Pbkdf2Params};

use super::FormOwner;
use super::error::SecureStringError;
use super::storage::{load_string, save_string, create_storage_key};
use super::string_ops::generate_salt;
use crate::utils::convert_types::{string_to_uint8array, uint8array_to_string};

const KEY_USAGE_DERIVE_KEY: &str = "deriveKey";
const KEY_USAGE_ENCRYPT: &str = "encrypt";
const KEY_USAGE_DECRYPT: &str = "decrypt";

type SecureStringResult<T> = Result<T, SecureStringError>;

pub fn get_crypto_subtle(
) -> SecureStringResult<(web_sys::Window, web_sys::Crypto, web_sys::SubtleCrypto)>
{
    let window = web_sys::window().ok_or(SecureStringError::NoWindow)?;
    let crypto = window.crypto().map_err(|_| SecureStringError::NoCrypto)?;
    let subtle = crypto.subtle();
    Ok((window, crypto, subtle))
}

pub async fn encrypt(
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

pub async fn decrypt(
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

pub async fn derive_crypto_key(
    password: &str,
    salt: &str,
) -> SecureStringResult<CryptoKey> {
    let subtle = get_crypto_subtle()?.2;

    let password_data = string_to_uint8array(password);
    let salt_data = string_to_uint8array(salt);

    let password_key =
        import_key(&subtle, &password_data, &[KEY_USAGE_DERIVE_KEY]).await?;
    let key_usages = [KEY_USAGE_ENCRYPT, KEY_USAGE_DECRYPT];
    let derived_key =
        derive_key(&subtle, &salt_data, &password_key, &key_usages).await?;

    Ok(derived_key)
}

async fn import_key(
    subtle: &web_sys::SubtleCrypto,
    password_data: &Uint8Array,
    key_usages: &[&str],
) -> SecureStringResult<CryptoKey> {
    let key_usages_js = key_usages_to_js(key_usages);

    let password_key_promise = subtle.import_key_with_str(
        "raw",
        password_data,
        "PBKDF2",
        false,
        &key_usages_js.into(),
    )?;

    let password_key: CryptoKey =
        wasm_bindgen_futures::JsFuture::from(password_key_promise)
            .await?
            .dyn_into()?;
    Ok(password_key)
}

async fn derive_key(
    subtle: &web_sys::SubtleCrypto,
    salt_data: &Uint8Array,
    password_key: &CryptoKey,
    key_usages: &[&str],
) -> SecureStringResult<CryptoKey> {
    const ITERATIONS: u32 = 100000;
    const KEY_LENGTH: u16 = 256;
    let key_usages_js = key_usages_to_js(key_usages);

    let derived_key_promise = subtle.derive_key_with_object_and_object(
        &Pbkdf2Params::new(
            "PBKDF2",
            &JsValue::from_str("SHA-256"),
            ITERATIONS,
            &(*salt_data).clone().into(),
        ),
        password_key,
        &AesKeyGenParams::new("AES-GCM", KEY_LENGTH),
        true,
        &key_usages_js.into(),
    )?;
    let derived_key: CryptoKey =
        wasm_bindgen_futures::JsFuture::from(derived_key_promise)
            .await?
            .dyn_into()?;
    Ok(derived_key)
}

fn key_usages_to_js(key_usages: &[&str]) -> js_sys::Array {
    let arr = js_sys::Array::new();
    for usage in key_usages {
        arr.push(&JsValue::from_str(usage));
    }
    arr
}

pub async fn derive_key_from_password(
    hashed_username: &str,
    password: &str,
) -> SecureStringResult<CryptoKey> {
    let form_owner = FormOwner {
        tag: "USER".to_string(),
        id: hashed_username.to_string(),
    };
    let storage_key = create_storage_key(&form_owner);

    let salt = match load_string(&storage_key).await {
        Some(salt) => salt,
        None => {
            let new_salt = generate_salt()?;
            save_string(&storage_key, &new_salt).await?;
            new_salt
        }
    };
    derive_crypto_key(password, &salt).await
}

pub fn hash_username(user: &str) -> String {
    let hash = hash(user.as_bytes());
    hash.to_hex().to_string()
}
