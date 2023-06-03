use std::collections::HashMap;

use serde_json;

use super::encryption::derive_crypto_key;
use super::error::SecureStringError;
use super::secure_storage::SecureStorage;
use super::string_ops::generate_password_base64;
use super::{ObjectKey, SecureStringResult};

#[derive(Debug, Clone, PartialEq)]
pub struct Configurations {}

impl Configurations {
    pub async fn list(
        &self,
        secure_storage: &SecureStorage,
    ) -> SecureStringResult<HashMap<String, String>> {
        // Load the stored form metadata.
        let forms_db: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&secure_storage.load().await?)?;

        // Transform the metadata into the desired output format.
        let configurations = forms_db
            .into_iter()
            .map(|(id, meta)| {
                // Extract the name of the form, using "Unknown" as the default.
                let name = meta
                    .get("NAME")
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());

                // Return a tuple of the form id and name.
                (id, name)
            })
            .collect();

        Ok(configurations)
    }

    pub async fn save(
        &self,
        secure_storage: &mut SecureStorage,
        object_key: ObjectKey,
        config: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        let form_name = config
            .get("__NAME__")
            .unwrap_or(&"Unknown".to_string())
            .clone();
        let form_id = &object_key.id;
        let password = generate_password_base64()?;
        let derived_key = derive_crypto_key(&password, form_id).await?;
        let config_json = serde_json::to_string(&config)?;

        // secure storage for the form
        let secure_storage_form =
            SecureStorage::new(object_key.clone(), derived_key);
        secure_storage_form.save(&config_json).await?;

        let mut forms_db: HashMap<String, HashMap<String, String>> =
            match secure_storage.load().await {
                Ok(passwords_json) => serde_json::from_str(&passwords_json)?,
                Err(_) => HashMap::new(),
            };
        let mut form_config = HashMap::new();
        form_config.insert("NAME".to_string(), form_name);
        form_config.insert("PASSWD".to_string(), password);
        forms_db.insert(form_id.to_string(), form_config);

        secure_storage
            .save(&serde_json::to_string(&forms_db)?)
            .await
    }

    // Loads the configuration securely after decrypting it with a derived key
    pub async fn load(
        &self,
        secure_storage: &SecureStorage,
        object_key: ObjectKey,
    ) -> SecureStringResult<HashMap<String, String>> {
        let object_id = &object_key.id.clone();
        let meta: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&secure_storage.load().await?)?;
        let meta =
            meta.get(object_id)
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Password for {} not found",
                    object_id
                )))?;
        let password =
            meta.get("PASSWD")
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Password for {} not found",
                    object_id
                )))?;

        let derived_key = derive_crypto_key(&password, object_id).await?;
        let secure_storage_form =
            SecureStorage::new(object_key.clone(), derived_key);
        let config_json = secure_storage_form.load().await?;
        let config: HashMap<String, String> =
            serde_json::from_str(&config_json)
                .map_err(SecureStringError::from)?;
        Ok(config)
    }

    pub async fn add(
        &mut self,
        secure_storage: &SecureStorage,
        object_key: ObjectKey,
        name: String,
    ) -> SecureStringResult<()> {
        let form_id = &object_key.id.clone();
        let mut forms_db: HashMap<String, HashMap<String, String>> =
            match secure_storage.load().await {
                Ok(passwords_json) => serde_json::from_str(&passwords_json)?,
                Err(_) => HashMap::new(),
            };
        let form_config = forms_db
            .entry(form_id.to_string())
            .or_insert_with(HashMap::new);
        form_config.insert("NAME".to_string(), name);
        secure_storage
            .save(&serde_json::to_string(&forms_db)?)
            .await?;
        Ok(())
    }

    pub async fn delete(
        &mut self,
        secure_storage: &SecureStorage,
        object_key: ObjectKey,
    ) -> SecureStringResult<()> {
        let form_id = &object_key.id.clone();

        // Load the stored form metadata.
        let mut forms_db: HashMap<String, HashMap<String, String>> =
            match secure_storage.load().await {
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
        secure_storage
            .save(&serde_json::to_string(&forms_db)?)
            .await?;

        Ok(())
    }
}
