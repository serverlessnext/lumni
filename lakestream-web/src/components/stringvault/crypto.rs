use web_sys::CryptoKey;

use super::encryption::derive_crypto_key;
use super::error::SecureStringError;
use super::storage::{create_storage_key, load_string, save_string};
use super::string_ops::generate_salt;
use super::ObjectKey;

type SecureStringResult<T> = Result<T, SecureStringError>;

pub fn get_crypto_subtle(
) -> SecureStringResult<(web_sys::Window, web_sys::Crypto, web_sys::SubtleCrypto)>
{
    let window = web_sys::window().ok_or(SecureStringError::NoWindow)?;
    let crypto = window.crypto().map_err(|_| SecureStringError::NoCrypto)?;
    let subtle = crypto.subtle();
    Ok((window, crypto, subtle))
}

pub async fn derive_key_from_password(
    hashed_username: &str,
    password: &str,
) -> SecureStringResult<CryptoKey> {
    let object_key = ObjectKey {
        tag: "USER".to_string(),
        id: hashed_username.to_string(),
    };
    let storage_key = create_storage_key(&object_key);

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
