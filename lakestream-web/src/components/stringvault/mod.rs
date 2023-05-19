use std::collections::HashMap;

use leptos::log;
use serde_json;
use wasm_bindgen::JsValue;
use web_sys::CryptoKey;
mod error;
mod helpers;

pub use error::SecureStringError;
use helpers::{
    derive_crypto_key, get_or_generate_salt, load_secure_string,
    save_secure_string,
};

type SecureStringResult<T> = Result<T, SecureStringError>;

#[derive(Clone, PartialEq, Debug)]
pub struct StringVault {
    key: CryptoKey,
}

impl StringVault {
    pub async fn new(user: &str, password: &str) -> Result<Self, JsValue> {
        let salt = get_or_generate_salt(user).await?;

        match derive_crypto_key(password, &salt).await {
            Ok(crypto_key) => Ok(Self { key: crypto_key }),
            Err(err) => Err(err),
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
        save_secure_string(&key, &config_json, &self.key).await
    }

    pub async fn load_secure_configuration(
        &self,
        uuid: &str,
    ) -> SecureStringResult<HashMap<String, String>> {
        let key = format!("SECRETS_{}", uuid);
        let config_json = load_secure_string(&key, &self.key).await?;
        let config: HashMap<String, String> =
            serde_json::from_str(&config_json)
                .map_err(SecureStringError::from)?;
        Ok(config)
    }
}
