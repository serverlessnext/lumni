use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{AesKeyGenParams, CryptoKey, Pbkdf2Params, SubtleCrypto};

use super::error::SecureStringError;

pub async fn import_key(
    subtle: &SubtleCrypto,
    password_data: &Uint8Array,
    key_usages: &[&str],
) -> Result<CryptoKey, SecureStringError> {
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

// New function to create Pbkdf2Params
fn create_pbkdf2_params(salt_data: &Uint8Array) -> Pbkdf2Params {
    const ITERATIONS: u32 = 100000;
    Pbkdf2Params::new(
        "PBKDF2",
        &JsValue::from_str("SHA-256"),
        ITERATIONS,
        &(*salt_data).clone().into(),
    )
}

// New function to create AesKeyGenParams
fn create_aes_key_gen_params() -> AesKeyGenParams {
    const KEY_LENGTH: u16 = 256;
    AesKeyGenParams::new("AES-GCM", KEY_LENGTH)
}

// Modify derive_key to use new helper functions
pub async fn derive_key(
    subtle: &SubtleCrypto,
    salt_data: &Uint8Array,
    password_key: &CryptoKey,
    key_usages: &[&str],
) -> Result<CryptoKey, SecureStringError> {
    let pbkdf2_params = create_pbkdf2_params(salt_data);
    let aes_key_gen_params = create_aes_key_gen_params();
    let key_usages_js = key_usages_to_js(key_usages);

    let derived_key_promise = subtle.derive_key_with_object_and_object(
        &pbkdf2_params,
        password_key,
        &aes_key_gen_params,
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

#[cfg(test)]
mod tests {
    use js_sys::Uint8Array;
    use wasm_bindgen_test::*;

    use super::*;
    use crate::crypto::get_crypto_subtle;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_key_usages_to_js() {
        let key_usages = ["encrypt", "decrypt"];
        let js_array = key_usages_to_js(&key_usages);

        assert_eq!(js_array.length(), 2);
        assert_eq!(js_array.get(0), JsValue::from_str("encrypt"));
        assert_eq!(js_array.get(1), JsValue::from_str("decrypt"));
    }

    #[wasm_bindgen_test]
    async fn test_import_key() {
        let (_, _, subtle) = get_crypto_subtle().unwrap();
        let key_usages = ["deriveKey"];
        let password_data = Uint8Array::from(&[1, 2, 3, 4][..]);

        let result = import_key(&subtle, &password_data, &key_usages).await;
        // Expect import_key to succeed
        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_create_pbkdf2_params() {
        let salt_data = Uint8Array::from(&[1, 2, 3, 4][..]);
        let params = create_pbkdf2_params(&salt_data);
        let debug_output = format!("{:?}", params);

        // while not ideal to use debug, Pbkdf2Params does not have getters
        // so this is the next best thing to test the values
        assert!(debug_output.contains("\"PBKDF2\""));
        assert!(debug_output.contains("\"SHA-256\""));
        assert!(debug_output.contains("100000"));
    }

    #[wasm_bindgen_test]
    fn test_create_aes_key_gen_params() {
        let params = create_aes_key_gen_params();
        let debug_output = format!("{:?}", params);

        // while not ideal to use debug, Pbkdf2Params does not have getters
        // so this is the next best thing to test the values
        assert!(debug_output.contains("\"AES-GCM\""));
        assert!(debug_output.contains("256"));
    }

    #[wasm_bindgen_test]
    async fn test_derive_key() {
        const KEY_USAGE_ENCRYPT: &str = "encrypt";
        const KEY_USAGE_DECRYPT: &str = "decrypt";

        let (_, _, subtle) = get_crypto_subtle().unwrap();
        let key_usages_import = ["deriveKey"];
        let key_usages_derive = [KEY_USAGE_ENCRYPT, KEY_USAGE_DECRYPT];
        let password_data = Uint8Array::from(&[1, 2, 3, 4][..]);
        let salt_data = Uint8Array::from(&[5, 6, 7, 8, 9, 10, 11, 12][..]);

        let password_key =
            match import_key(&subtle, &password_data, &key_usages_import).await
            {
                Ok(key) => key,
                Err(err) => panic!("import_key failed with error: {:?}", err),
            };

        match derive_key(&subtle, &salt_data, &password_key, &key_usages_derive)
            .await
        {
            Ok(_) => (),
            Err(err) => panic!("derive_key failed with error: {:?}", err),
        };
    }
}
