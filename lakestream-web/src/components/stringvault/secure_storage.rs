use base64::engine::general_purpose;
use base64::Engine as _;
use js_sys::Uint8Array;
use leptos::log;
use web_sys::CryptoKey;

use super::crypto::{decrypt, encrypt, get_crypto_subtle};
use super::storage::{
    create_storage_key, delete_string, load_string, save_string,
};
use super::{FormOwner, SecureStringError, SecureStringResult};

#[derive(Debug, Clone, PartialEq)]
pub struct SecureStorage {
    form_owner: FormOwner,
    crypto_key: Option<CryptoKey>,
}

impl SecureStorage {
    pub fn new(form_owner: FormOwner, crypto_key: CryptoKey) -> Self {
        Self {
            form_owner,
            crypto_key: Some(crypto_key),
        }
    }

    pub async fn exists(form_owner: FormOwner) -> bool {
        let storage_key = create_storage_key(&form_owner);
        load_string(&storage_key).await.is_some()
    }

    pub fn form_owner(&self) -> &FormOwner {
        &self.form_owner
    }

    pub fn for_deletion(form_owner: FormOwner) -> Self {
        Self {
            form_owner,
            crypto_key: None,
        }
    }

    pub async fn save(&self, value: &str) -> SecureStringResult<()> {
        let crypto_key = self
            .crypto_key
            .as_ref()
            .ok_or_else(|| SecureStringError::InvalidCryptoKey)?;

        let (_, crypto, _) = get_crypto_subtle()?;

        let mut iv = [0u8; 12];
        crypto.get_random_values_with_u8_array(&mut iv)?;
        let (encrypted_data, iv_vec) = encrypt(crypto_key, value, &iv).await?;

        let encrypted_data = js_sys::Uint8Array::new(&encrypted_data);
        let encrypted_data: Vec<u8> = encrypted_data.to_vec();
        let encrypted_data_with_iv = [iv_vec, encrypted_data].concat();
        let encrypted_data_with_iv_base64 =
            general_purpose::STANDARD.encode(&encrypted_data_with_iv);

        let storage_key = create_storage_key(&self.form_owner);
        save_string(&storage_key, &encrypted_data_with_iv_base64)
            .await
            .map_err(SecureStringError::from)
    }

    pub async fn load(&self) -> SecureStringResult<String> {
        let crypto_key = self
            .crypto_key
            .as_ref()
            .ok_or_else(|| SecureStringError::InvalidCryptoKey)?;
        let storage_key = create_storage_key(&self.form_owner);
        let encrypted_data_base64 = load_string(&storage_key)
            .await
            .ok_or(SecureStringError::NoLocalStorageData)?;
        let encrypted_data_with_iv =
            general_purpose::STANDARD.decode(&encrypted_data_base64)?;
        let (iv, encrypted_data) = encrypted_data_with_iv.split_at(12);
        let encrypted_data = Uint8Array::from(&encrypted_data[..]);
        let iv = Uint8Array::from(&iv[..]);

        let decrypted_data = decrypt(crypto_key, &encrypted_data, &iv)
            .await
            .map_err(|_| {
                SecureStringError::DecryptError(
                    "Please ensure the password is correct.".to_owned(),
                )
            })?;
        Ok(decrypted_data)
    }

    pub async fn delete(&self) -> SecureStringResult<()> {
        let storage_key = create_storage_key(&self.form_owner);
        delete_string(&storage_key)
            .await
            .map_err(SecureStringError::from)
    }

    pub async fn update(&self, new_value: &str) -> SecureStringResult<()> {
        log!("Updating secure string");
        // Delete the existing string
        // self.delete().await?;
        // Save the new value
        // self.save(new_value).await?;
        Ok(())
    }
}
