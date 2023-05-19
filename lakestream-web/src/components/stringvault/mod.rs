
use std::collections::HashMap;
use web_sys::CryptoKey;
use serde_json;

mod helpers;
mod error;

pub use error::SecureStringError;
use helpers::{load_secure_string, save_secure_string};


type SecureStringResult<T> = Result<T, SecureStringError>;



#[derive(Clone, PartialEq, Debug)]
pub struct StringVault {
    key: CryptoKey,
}

impl StringVault {
    pub fn new(key: CryptoKey) -> Self {
        Self {
            key,
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
        let config_json = serde_json::to_string(&config)?;
        let key = format!("SECRETS_{}", uuid);
        save_secure_string(&key, &config_json, &self.key)
            .await
    }

    pub async fn load_secure_configuration(
        &self,
        uuid: &str,
    ) -> SecureStringResult<HashMap<String, String>> {
        let key = format!("SECRETS_{}", uuid);
        let config_json = load_secure_string(&key, &self.key).await?;
        let config: HashMap<String, String> = serde_json::from_str(&config_json)
            .map_err(SecureStringError::from)?;
        Ok(config)
    }
}

