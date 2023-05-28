mod crypto;
mod error;
mod form_handler;
mod form_input;
mod form_input_builder;
mod form_view;
mod secure_storage;
mod storage;
mod string_ops;
use std::collections::HashMap;

use crypto::{derive_crypto_key, derive_key_from_password, hash_username};
pub use error::SecureStringError;
pub use form_handler::{ConfigManager, FormHandler, handle_form_submission};
pub use form_input::{
    create_input_elements, FormInputField, InputData, InputElementOpts,
    InputElements, InputFieldView,
};
pub use form_input_builder::FormInputFieldBuilder;
pub use form_view::FormView;
use leptos::log;
use secure_storage::SecureStorage;
use serde_json;
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
    secure_storage: SecureStorage,
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
        let form_owner = FormOwner {
            tag: hashed_username.clone(),
            id: "self".to_string(),
        };
        let secure_storage = SecureStorage::new(form_owner, crypto_key);
        Ok(Self {
            secure_storage,
            username: username.to_string(),
            hashed_username,
        })
    }

    pub async fn new_and_validate(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        let vault = StringVault::new(username, password).await?;

        // Try to load the passwords to validate the password
        match vault.secure_storage.load().await {
            Ok(contents) => {
                log!("Contents: {}", contents);
                Ok(vault)
            },
            Err(err) => match err {
                SecureStringError::NoLocalStorageData => {
                    // user is not yet created
                    log!("New user create");
                    Ok(vault)
                }
                SecureStringError::DecryptError(_) => {
                    // user exists but password is wrong
                    // TODO: offer reset password option
                    Err(err)
                }
                _ => Err(err), // Propagate any other errors
            },
        }
    }

    pub fn set_admin_key(&mut self, new_key: CryptoKey) {
        let form_owner = self.secure_storage.form_owner().clone();
        self.secure_storage = SecureStorage::new(form_owner, new_key);
    }

    pub async fn list_configurations(
        &self,
    ) -> SecureStringResult<Vec<(String, String)>> {
        // Load the stored form metadata.
        let forms_db: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&self.secure_storage.load().await?)?;

        // Transform the metadata into the desired output format.
        let configurations = forms_db
            .into_iter()
            .map(|(id, meta)| {
                // Extract the name of the form, using "Unknown" as the default.
                let name = meta.get("NAME").unwrap_or(&"Unknown".to_string()).clone();

                // Return a tuple of the form id and name.
                (id, name)
            })
            .collect();

        Ok(configurations)
    }


    pub async fn save_secure_configuration(
        &mut self,
        form_owner: FormOwner,
        config: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        let form_name = config.get("__NAME__").unwrap_or(&"Unknown".to_string()).clone();
        let form_id = &form_owner.id.clone();
        let password = generate_password()?;
        let derived_key = derive_crypto_key(&password, form_id).await?;
        let config_json = serde_json::to_string(&config)?;

        let secure_storage =
            SecureStorage::new(form_owner.clone(), derived_key);
        secure_storage.save(&config_json).await?;

        let mut forms_db: HashMap<String, HashMap<String, String>> =
            match self.secure_storage.load().await {
                Ok(passwords_json) => serde_json::from_str(&passwords_json)?,
                Err(_) => HashMap::new(),
            };
        let mut form_config = HashMap::new();
        form_config.insert("NAME".to_string(), form_name);
        form_config.insert("PASSWD".to_string(), password);
        forms_db.insert(form_id.to_string(), form_config);
        self.secure_storage
            .save(&serde_json::to_string(&forms_db)?)
            .await
    }

    // Loads the configuration securely after decrypting it with a derived key
    pub async fn load_secure_configuration(
        &self,
        form_owner: FormOwner,
    ) -> SecureStringResult<HashMap<String, String>> {
        let form_id = &form_owner.id.clone();
        let meta: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&self.secure_storage.load().await?)?;
        let meta = meta.get(form_id).ok_or(
            SecureStringError::PasswordNotFound(format!(
                "Password for {} not found",
                form_id
            )),
        )?;
        let password = meta.get("PASSWD").ok_or(
            SecureStringError::PasswordNotFound(format!(
                "Password for {} not found",
                form_id
            )),
        )?;

        let derived_key = derive_crypto_key(&password, form_id).await?;
        let secure_storage =
            SecureStorage::new(form_owner.clone(), derived_key);
        let config_json = secure_storage.load().await?;
        let config: HashMap<String, String> =
            serde_json::from_str(&config_json)
                .map_err(SecureStringError::from)?;
        Ok(config)
    }

    pub async fn delete_configuration(
        &mut self,
        form_owner: FormOwner,
    ) -> SecureStringResult<()> {
        let form_id = &form_owner.id.clone();

        // Load the stored form metadata.
        let mut forms_db: HashMap<String, HashMap<String, String>> =
            match self.secure_storage.load().await {
                Ok(passwords_json) => serde_json::from_str(&passwords_json)?,
                Err(_) => HashMap::new(),
            };

        // Remove the specific configuration
        if forms_db.remove(form_id).is_none() {
            return Err(SecureStringError::PasswordNotFound(format!(
                "Configuration for {} not found",
                form_id
            )));
        }

        // Save the updated form metadata back to the vault.
        self.secure_storage
            .save(&serde_json::to_string(&forms_db)?)
            .await?;

        Ok(())
    }


    pub async fn reset_vault(username: &str) -> SecureStringResult<()> {
        let hashed_username = hash_username(username);
        let form_owner = FormOwner {
            tag: hashed_username.clone(),
            id: "self".to_string(),
        };

        let secure_storage = SecureStorage::for_deletion(form_owner);
        secure_storage.delete().await?;

        Ok(())
    }
}
