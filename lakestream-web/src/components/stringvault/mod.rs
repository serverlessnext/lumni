pub mod config_handler;
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
use web_sys::CryptoKey;

pub type SecureStringResult<T> = Result<T, SecureStringError>;

#[derive(Clone, PartialEq, Debug)]
pub struct StringVault {
    key: CryptoKey,
    hashed_username: String,
}

const EMPTY_SALT: &str = "";

impl StringVault {
    pub async fn new(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        let hashed_username = hash_username(username);
        let crypto_key =
            derive_key_from_password(&hashed_username, password).await?;
        Ok(Self {
            hashed_username,
            key: crypto_key,
        })
    }

    pub fn set_admin_key(&mut self, new_key: CryptoKey) {
        self.key = new_key;
    }

    // Saves the configuration securely after encrypting it with a derived key
    pub async fn save_secure_configuration(
        &mut self,
        uuid: &str,
        config: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        let password = generate_password()?;
        let derived_key = derive_crypto_key(&password, EMPTY_SALT).await?;
        let config_json = serde_json::to_string(&config)?;
        save_secure_string(uuid, &config_json, &derived_key).await?;

        let mut passwords = match self.load_passwords().await {
            Ok(passwords) => passwords,
            Err(_) => HashMap::new(),
        };

        passwords.insert(uuid.to_string(), password);
        self.save_passwords(passwords).await
    }

    // Loads the configuration securely after decrypting it with a derived key
    pub async fn load_secure_configuration(
        &self,
        uuid: &str,
    ) -> SecureStringResult<HashMap<String, String>> {
        let passwords = self.load_passwords().await?;
        let password =
            passwords
                .get(uuid)
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Password for {} not found",
                    uuid
                )))?;

        let derived_key = derive_crypto_key(&password, "").await?;
        let config_json = load_secure_string(&uuid, &derived_key).await?;
        let config: HashMap<String, String> =
            serde_json::from_str(&config_json)
                .map_err(SecureStringError::from)?;

        Ok(config)
    }

    // Saves the password map after encrypting it with the vault key
    async fn save_passwords(
        &mut self,
        passwords: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        let passwords_json = serde_json::to_string(&passwords)?;
        save_secure_string(&self.hashed_username, &passwords_json, &self.key)
            .await
    }

    // Loads the password map after decrypting it with the vault key
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
