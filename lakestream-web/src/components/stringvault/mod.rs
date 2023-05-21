pub mod crypto;
pub mod error;
pub mod storage;
pub mod string_ops;
use std::collections::HashMap;

use crypto::{derive_crypto_key, derive_key_from_password, hash_username};
pub use error::SecureStringError;
use serde_json;
use storage::{load_secure_string, save_secure_string};
use string_ops::generate_password;
use wasm_bindgen::JsValue;
use web_sys::CryptoKey;

pub type SecureStringResult<T> = Result<T, SecureStringError>;

#[derive(Clone, PartialEq, Debug)]
pub struct StringVault {
    key: CryptoKey,
    hashed_username: String,
}

const EMPTY_SALT: &str = "";

impl StringVault {
    pub async fn new(username: &str, password: &str) -> Result<Self, JsValue> {
        let hashed_username = hash_username(username);
        match derive_key_from_password(&hashed_username, password).await {
            Ok(crypto_key) => Ok(Self {
                hashed_username,
                key: crypto_key,
            }),
            Err(err) => Err(JsValue::from_str(&err.to_string())),
        }
    }

    pub fn set_admin_key(&mut self, new_key: CryptoKey) {
        self.key = new_key;
    }

    pub async fn save_secure_configuration(
        &mut self,
        uuid: &str,
        config: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        // Generate a new password for the configuration
        let password = generate_password()?; // you need to implement this function

        // Derive a new key from the password
        let derived_key = derive_crypto_key(&password, EMPTY_SALT).await?;

        // Encrypt and store the configuration with the derived key
        let config_json = serde_json::to_string(&config)?;
        save_secure_string(uuid, &config_json, &derived_key).await?;

        // Load the encrypted passwords map
        let mut passwords = match self.load_passwords().await {
            Ok(passwords) => passwords,
            Err(_) => HashMap::new(),
        };

        // Update the passwords map and save it
        passwords.insert(uuid.to_string(), password);
        self.save_passwords(passwords).await
    }

    pub async fn load_secure_configuration(
        &self,
        uuid: &str,
    ) -> SecureStringResult<HashMap<String, String>> {
        // Load the encrypted passwords map
        let passwords = self.load_passwords().await?;

        // Get the password for the configuration
        let password =
            passwords
                .get(uuid)
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Password for {} not found",
                    uuid
                )))?;

        // Derive the key from the loaded password
        let derived_key = derive_crypto_key(&password, "").await?;

        // Load the configuration with the derived key
        let config_json = load_secure_string(&uuid, &derived_key).await?;
        let config: HashMap<String, String> =
            serde_json::from_str(&config_json)
                .map_err(SecureStringError::from)?;

        Ok(config)
    }

    async fn save_passwords(
        &mut self,
        passwords: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        let passwords_json = serde_json::to_string(&passwords)?;
        save_secure_string(&self.hashed_username, &passwords_json, &self.key)
            .await
    }

    async fn load_passwords(
        &self,
    ) -> SecureStringResult<HashMap<String, String>> {
        let passwords_json =
            load_secure_string(&self.hashed_username, &self.key).await?;
        let passwords: HashMap<String, String> =
            serde_json::from_str(&passwords_json)
                .map_err(SecureStringError::from)?;
        Ok(passwords)
    }
}
