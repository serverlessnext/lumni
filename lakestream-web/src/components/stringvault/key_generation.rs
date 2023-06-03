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

pub async fn derive_key(
    subtle: &SubtleCrypto,
    salt_data: &Uint8Array,
    password_key: &CryptoKey,
    key_usages: &[&str],
) -> Result<CryptoKey, SecureStringError> {
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
