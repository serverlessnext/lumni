use blake3::hash;
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::JsCast;
use web_sys::{AesGcmParams, CryptoKey};

use super::convert_types::{string_to_uint8array, uint8array_to_string};
use super::crypto::get_crypto_subtle;
use super::error::SecureStringError;
use super::key_generation::{derive_key, import_key};

const KEY_USAGE_DERIVE_KEY: &str = "deriveKey";
const KEY_USAGE_ENCRYPT: &str = "encrypt";
const KEY_USAGE_DECRYPT: &str = "decrypt";

type SecureStringResult<T> = Result<T, SecureStringError>;

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
    if password.is_empty() {
        return Err(SecureStringError::EmptyPassword);
    }
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

pub fn hash_username(user: &str) -> String {
    let hash = hash(user.as_bytes());
    hash.to_hex().to_string()
}
