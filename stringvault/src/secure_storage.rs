use base64::engine::general_purpose;
use base64::Engine as _;
use js_sys::Uint8Array;
use web_sys::CryptoKey;

use super::crypto::get_crypto_subtle;
use super::encryption::{decrypt, encrypt};
use super::storage::{
    create_storage_key, delete_string, load_string, save_string,
};
use super::{ObjectKey, SecureStringError, SecureStringResult};

#[derive(Debug, Clone, PartialEq)]
pub struct SecureStorage {
    object_key: ObjectKey,
    crypto_key: Option<CryptoKey>,
}

impl SecureStorage {
    pub fn new(object_key: ObjectKey, crypto_key: CryptoKey) -> Self {
        Self {
            object_key,
            crypto_key: Some(crypto_key),
        }
    }

    pub async fn exists(object_key: ObjectKey) -> bool {
        let storage_key = create_storage_key(&object_key);
        load_string(&storage_key).await.is_some()
    }

    pub fn object_key(&self) -> &ObjectKey {
        &self.object_key
    }

    pub fn for_deletion(object_key: ObjectKey) -> Self {
        Self {
            object_key,
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

        let storage_key = create_storage_key(&self.object_key);
        save_string(&storage_key, &encrypted_data_with_iv_base64)
            .await
            .map_err(SecureStringError::from)
    }

    pub async fn load(&self) -> SecureStringResult<String> {
        let crypto_key = self
            .crypto_key
            .as_ref()
            .ok_or_else(|| SecureStringError::InvalidCryptoKey)?;
        let storage_key = create_storage_key(&self.object_key);
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
        let storage_key = create_storage_key(&self.object_key);
        delete_string(&storage_key)
            .await
            .map_err(SecureStringError::from)
    }

    pub async fn empty(&self) -> SecureStringResult<()> {
        let empty_hashmap = serde_json::json!({}).to_string();
        self.save(&empty_hashmap).await
    }

    #[allow(unused)]
    pub async fn update(&self, new_value: &str) -> SecureStringResult<()> {
        // Delete the existing string
        // self.delete().await?;
        // Save the new value
        // self.save(new_value).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;
    use crate::crypto::derive_key_from_password;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_new() {
        let object_key = ObjectKey::new("test1", "test_id1").unwrap();
        let password = "password_for_new";

        let crypto_key_result =
            derive_key_from_password(&object_key, password).await;
        assert!(crypto_key_result.is_ok());

        let crypto_key = crypto_key_result.unwrap();
        let secure_storage =
            SecureStorage::new(object_key.clone(), crypto_key.clone());
        assert_eq!(secure_storage.object_key, object_key);
        assert_eq!(secure_storage.crypto_key, Some(crypto_key));
    }

    #[wasm_bindgen_test]
    fn test_for_deletion() {
        let object_key = ObjectKey::new("test2", "test_id2").unwrap();

        let secure_storage = SecureStorage::for_deletion(object_key.clone());
        assert_eq!(secure_storage.object_key, object_key);
        assert_eq!(secure_storage.crypto_key, None);
    }

    #[wasm_bindgen_test]
    async fn test_save_load_delete() {
        let object_key = ObjectKey::new("test3", "test_id3").unwrap();
        let password = "password_for_save_load_delete";
        let value = "test_value_for_save_load_delete";

        // Create the crypto key
        let crypto_key_result =
            derive_key_from_password(&object_key, password).await;
        assert!(crypto_key_result.is_ok());

        let crypto_key = crypto_key_result.unwrap();
        let secure_storage =
            SecureStorage::new(object_key.clone(), crypto_key.clone());

        // Save the string
        let save_result = secure_storage.save(value).await;
        assert!(save_result.is_ok());

        // Load the string
        let load_result = secure_storage.load().await;
        assert!(load_result.is_ok());

        // Assert the loaded string is equal to the original one
        let loaded_value = load_result.unwrap();
        assert_eq!(loaded_value, value);

        // Delete the storage
        let delete_result = secure_storage.delete().await;
        assert!(delete_result.is_ok());

        // Assert that trying to load the string results in a SecureStringError::NoLocalStorageData error
        let load_result = secure_storage.load().await;
        assert!(load_result.is_err());
        assert_eq!(
            load_result.unwrap_err(),
            SecureStringError::NoLocalStorageData
        );
    }

    #[wasm_bindgen_test]
    async fn test_exists() {
        let object_key =
            ObjectKey::new("test_exists", "test_id_exists").unwrap();
        let password = "password_for_exists";
        let value = "test_value_for_exists";

        // Ensure the secure string does not exist yet
        let exists = SecureStorage::exists(object_key.clone()).await;
        assert_eq!(exists, false);

        // Create the crypto key
        let crypto_key_result =
            derive_key_from_password(&object_key, password).await;
        assert!(crypto_key_result.is_ok());

        let crypto_key = crypto_key_result.unwrap();
        let secure_storage =
            SecureStorage::new(object_key.clone(), crypto_key.clone());

        // Save the string
        let save_result = secure_storage.save(value).await;
        assert!(save_result.is_ok());

        // Ensure the secure string now exists
        let exists = SecureStorage::exists(object_key.clone()).await;
        assert_eq!(exists, true);

        // Delete the storage
        let delete_result = secure_storage.delete().await;
        assert!(delete_result.is_ok());

        // Ensure the secure string no longer exists
        let exists = SecureStorage::exists(object_key.clone()).await;
        assert_eq!(exists, false);
    }

    #[wasm_bindgen_test]
    async fn test_empty() {
        let object_key = ObjectKey::new("test_empty", "test_id_empty").unwrap();
        let password = "password_for_empty";
        let value = "test_value_for_empty";

        // Create the crypto key
        let crypto_key_result =
            derive_key_from_password(&object_key, password).await;
        assert!(crypto_key_result.is_ok());

        let crypto_key = crypto_key_result.unwrap();
        let secure_storage =
            SecureStorage::new(object_key.clone(), crypto_key.clone());

        // Save the string
        let save_result = secure_storage.save(value).await;
        assert!(save_result.is_ok());

        // Empty the storage
        let empty_result = secure_storage.empty().await;
        assert!(empty_result.is_ok());

        // Load the string
        let load_result = secure_storage.load().await;
        assert!(load_result.is_ok());

        // Assert the loaded string is an empty JSON object (which is what `empty` function should do)
        let loaded_value = load_result.unwrap();
        assert_eq!(loaded_value, "{}");
    }

    #[wasm_bindgen_test]
    async fn test_for_deletion_no_key() {
        let object_key =
            ObjectKey::new("test_for_deletion", "test_id_for_deletion")
                .unwrap();

        let secure_storage = SecureStorage::for_deletion(object_key.clone());

        // Try to save a string
        let save_result = secure_storage.save("test_value").await;
        assert!(save_result.is_err());
        assert_eq!(
            save_result.unwrap_err(),
            SecureStringError::InvalidCryptoKey
        );

        // Try to load a string
        let load_result = secure_storage.load().await;
        assert!(load_result.is_err());
        assert_eq!(
            load_result.unwrap_err(),
            SecureStringError::InvalidCryptoKey
        );

        // Try to delete a string
        // delete is idempotent and does not need key, so it should not fail
        let delete_result = secure_storage.delete().await;
        assert!(delete_result.is_ok());
    }

    #[wasm_bindgen_test]
    async fn test_large_string() {
        let object_key =
            ObjectKey::new("test_large_string", "test_id_large_string")
                .unwrap();
        let password = "password";
        let value = "a".repeat(1_000_000);

        // Create the crypto key
        let crypto_key_result =
            derive_key_from_password(&object_key, password).await;
        assert!(crypto_key_result.is_ok());

        let crypto_key = crypto_key_result.unwrap();
        let secure_storage =
            SecureStorage::new(object_key.clone(), crypto_key.clone());

        // Save the string
        let save_result = secure_storage.save(&value).await;
        assert!(save_result.is_ok());

        // Load the string
        let load_result = secure_storage.load().await;
        assert!(load_result.is_ok());

        // Assert the loaded string is equal to the original one
        let loaded_value = load_result.unwrap();
        assert_eq!(loaded_value, value);
    }

    #[wasm_bindgen_test]
    async fn test_different_crypto_key() {
        let object_key = ObjectKey::new(
            "test_different_crypto_key",
            "test_id_different_crypto_key",
        )
        .unwrap();
        let password = "password";
        let value = "test_value";

        // Create the crypto key
        let crypto_key_result =
            derive_key_from_password(&object_key, password).await;
        assert!(crypto_key_result.is_ok());

        let crypto_key = crypto_key_result.unwrap();
        let secure_storage =
            SecureStorage::new(object_key.clone(), crypto_key.clone());

        // Save the string
        let save_result = secure_storage.save(value).await;
        assert!(save_result.is_ok());

        // Create a different crypto key
        let different_crypto_key_result =
            derive_key_from_password(&object_key, "different_password").await;
        assert!(different_crypto_key_result.is_ok());

        let different_crypto_key = different_crypto_key_result.unwrap();
        let different_secure_storage = SecureStorage::new(
            object_key.clone(),
            different_crypto_key.clone(),
        );

        // Try to load the string with the different crypto key
        let load_result = different_secure_storage.load().await;
        assert!(load_result.is_err());
        assert_eq!(
            load_result.unwrap_err(),
            SecureStringError::DecryptError(
                "Please ensure the password is correct.".to_owned()
            )
        );
    }

    #[wasm_bindgen_test]
    async fn test_special_characters_in_object_key() {
        let object_key = ObjectKey::new(
            "test_@$%^&*(){}[];:/?,<>'\"\\",
            "test_id_@$%^&*(){}[];:/?,<>'\"\\",
        )
        .unwrap();
        let password = "password";
        let value = "test_value";

        // Create the crypto key
        let crypto_key_result =
            derive_key_from_password(&object_key, password).await;
        assert!(crypto_key_result.is_ok());

        let crypto_key = crypto_key_result.unwrap();
        let secure_storage =
            SecureStorage::new(object_key.clone(), crypto_key.clone());

        // Save the string
        let save_result = secure_storage.save(value).await;
        assert!(save_result.is_ok());

        // Load the string
        let load_result = secure_storage.load().await;
        assert!(load_result.is_ok());

        // Assert the loaded string is equal to the original one
        let loaded_value = load_result.unwrap();
        assert_eq!(loaded_value, value);

        // Delete the storage
        let delete_result = secure_storage.delete().await;
        assert!(delete_result.is_ok());
    }
}
