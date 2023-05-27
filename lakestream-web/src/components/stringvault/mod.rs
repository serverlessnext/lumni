mod crypto;
mod error;
mod form_handler;
mod form_input;
mod form_input_builder;
mod form_view;
mod storage;
mod string_ops;

use std::collections::HashMap;

use crypto::{derive_crypto_key, derive_key_from_password, hash_username};
pub use error::SecureStringError;
pub use form_handler::{ConfigManager, FormHandler};
pub use form_input::{
    create_input_elements, FormInputField, InputData, InputElements,
    InputFieldView, InputElementOpts,
};
pub use form_input_builder::FormInputFieldBuilder;
pub use form_view::FormView;
use serde_json;
use storage::{load_secure_string, save_secure_string};
use string_ops::generate_password;
use web_sys::CryptoKey;


const EMPTY_SALT: &str = "";

pub type SecureStringResult<T> = Result<T, SecureStringError>;

#[derive(Clone, PartialEq, Debug)]
pub struct FormOwner {
    pub tag: String,
    pub id: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct StringVault {
    key: CryptoKey,
    username: String,
    hashed_username: String,
}

impl StringVault {
    pub async fn new(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        let hashed_username = hash_username(username);
        let crypto_key =
            derive_key_from_password(&hashed_username, password).await?;
        Ok(Self {
            key: crypto_key,
            username: username.to_string(),
            hashed_username,
        })
    }

    pub fn set_admin_key(&mut self, new_key: CryptoKey) {
        self.key = new_key;
    }

    // Saves the configuration securely after encrypting it with a derived key
    pub async fn save_secure_configuration(
        &mut self,
        form_owner: FormOwner,
        config: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        let password = generate_password()?;
        let derived_key = derive_crypto_key(&password, EMPTY_SALT).await?;
        let config_json = serde_json::to_string(&config)?;
        let form_id = &form_owner.id.clone();
        save_secure_string(form_owner, &config_json, &derived_key).await?;

        let mut passwords = match self.load_passwords().await {
            Ok(passwords) => passwords,
            Err(_) => HashMap::new(),
        };

        passwords.insert(form_id.to_string(), password);
        self.save_passwords(passwords).await
    }

    // Loads the configuration securely after decrypting it with a derived key
    pub async fn load_secure_configuration(
        &self,
        form_owner: FormOwner,
    ) -> SecureStringResult<HashMap<String, String>> {
        let form_id = &form_owner.id.clone();
        let passwords = self.load_passwords().await?;
        let password =
            passwords
                .get(form_id)
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Password for {} not found",
                    form_id
                )))?;

        let derived_key = derive_crypto_key(&password, "").await?;
        let config_json = load_secure_string(form_owner, &derived_key).await?;
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
        let form_owner = FormOwner {
            tag: self.hashed_username.to_string().clone(),
            id: "self".to_string(),
        };
        save_secure_string(form_owner, &passwords_json, &self.key)
            .await
    }

    // Loads the password map after decrypting it with the vault key
    async fn load_passwords(
        &self,
    ) -> SecureStringResult<HashMap<String, String>> {
        let form_owner = FormOwner {
            tag: self.hashed_username.to_string().clone(),
            id: "self".to_string(),
        };
        let passwords_json =
            load_secure_string(form_owner, &self.key).await?;
        let passwords: HashMap<String, String> =
            serde_json::from_str(&passwords_json)
                .map_err(SecureStringError::from)?;
        Ok(passwords)
    }
}
