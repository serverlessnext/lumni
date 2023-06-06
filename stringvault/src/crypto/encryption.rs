use blake3::hash;
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::JsCast;
use web_sys::{AesGcmParams, CryptoKey};

use super::key_generation::{derive_key, import_key};
use super::utils::get_crypto_subtle;
use crate::utils::string_to_uint8array;
use crate::SecureStringError;

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
) -> SecureStringResult<Vec<u8>> {
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

    // let decrypted_data = uint8array_to_string(&decrypted_data_vec);
    Ok(decrypted_data_vec)
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

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_hash_username() {
        let username = "test_username";
        let hashed_username = hash_username(&username);
        // Validate if the hash length is as expected
        assert_eq!(hashed_username.len(), 64);
    }

    #[wasm_bindgen_test]
    async fn test_derive_crypto_key() {
        let password = "password";
        let salt = "salt";
        let result = derive_crypto_key(&password, &salt).await;
        // Expect derive_crypto_key to succeed
        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    async fn test_derive_crypto_key_empty_password() {
        let password_empty = "";
        let salt = "salt";
        let result = derive_crypto_key(&password_empty, &salt).await;
        // Expect derive_crypto_key to fail due to empty password
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    async fn test_encrypt_and_decrypt() {
        let password = "password";
        let salt = "salt";
        let key_result = derive_crypto_key(&password, &salt).await;
        let key = key_result.unwrap();

        let data = "data to be encrypted";
        let iv = &[0u8; 12];
        let encrypt_result = encrypt(&key, &data, iv).await;
        assert!(encrypt_result.is_ok());

        let (encrypted_data, iv) = encrypt_result.unwrap();
        let encrypted_data = Uint8Array::new(&encrypted_data);
        let iv = Uint8Array::from(iv.as_slice());

        let decrypt_result = decrypt(&key, &encrypted_data, &iv).await;
        assert!(decrypt_result.is_ok());

        let decrypted_data = decrypt_result.unwrap();
        // Expect decrypted data to match original data
        assert_eq!(decrypted_data, data.as_bytes());
    }
}
