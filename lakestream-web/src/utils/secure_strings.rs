use base64::engine::general_purpose;
use base64::Engine as _;
use js_sys::{ArrayBuffer, Uint8Array};
use leptos::log;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{AesGcmParams, CryptoKey};

use super::convert_types::{string_to_uint8array, uint8array_to_string};
use super::local_storage::{load_data, save_data};

#[derive(Debug)]
pub enum SecureStringError {
    JsError(JsValue),
    Base64Error(base64::DecodeError),
    NoWindow,
    NoCrypto,
    NoLocalStorageData,
}

impl From<JsValue> for SecureStringError {
    fn from(e: JsValue) -> Self {
        SecureStringError::JsError(e)
    }
}

impl From<base64::DecodeError> for SecureStringError {
    fn from(e: base64::DecodeError) -> Self {
        SecureStringError::Base64Error(e)
    }
}

type SecureStringResult<T> = Result<T, SecureStringError>;

// Helper function
fn get_crypto_subtle(
) -> SecureStringResult<(web_sys::Window, web_sys::Crypto, web_sys::SubtleCrypto)>
{
    let window = web_sys::window().ok_or(SecureStringError::NoWindow)?;
    let crypto = window.crypto().map_err(|_| SecureStringError::NoCrypto)?;
    let subtle = crypto.subtle();
    Ok((window, crypto, subtle))
}

// Modified functions
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
    //let encrypted_data_with_iv_base64 = base64::encode(&encrypted_data_with_iv);
    let encrypted_data_with_iv_base64 =
        general_purpose::STANDARD.encode(&encrypted_data_with_iv);

    save_data(key, &encrypted_data_with_iv_base64).await?;
    Ok(())
}

pub async fn load_secure_string(
    key: &str,
    crypto_key: &CryptoKey,
) -> SecureStringResult<String> {
    let encrypted_data_base64 = load_data(key)
        .await
        .ok_or(SecureStringError::NoLocalStorageData)?;

    //let encrypted_data_with_iv = base64::decode(&encrypted_data_base64)?;
    let encrypted_data_with_iv =
        general_purpose::STANDARD.decode(&encrypted_data_base64)?;

    let (iv, encrypted_data) = encrypted_data_with_iv.split_at(12);
    let encrypted_data = Uint8Array::from(&encrypted_data[..]);
    let iv = Uint8Array::from(&iv[..]);

    let decrypted_data = decrypt(crypto_key, &encrypted_data, &iv).await?;
    log!("decrypted_data: {:?}", decrypted_data);
    Ok(decrypted_data)
}
