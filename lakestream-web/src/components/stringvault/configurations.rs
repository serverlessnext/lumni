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
        // Attempt to load the data.
        match secure_storage.load().await {
            // If successful, proceed with deserialization and transformation.
            Ok(data) => {
                // Load the stored form metadata.
                let forms_db: HashMap<String, HashMap<String, String>> =
                    serde_json::from_str(&data)?;

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
            // If loading fails with NoLocalStorageData, return an empty HashMap.
            Err(SecureStringError::NoLocalStorageData) => Ok(HashMap::new()),
            // For other errors, propagate the error.
            Err(err) => Err(err),
        }
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
        let form_id = &object_key.id();
        let password = generate_password_base64()?;
        let derived_key = derive_crypto_key(&password, form_id).await?;
        let config_json = serde_json::to_string(&config)?;

        // secure storage for the form
        let secure_storage_form =
            SecureStorage::new(object_key.clone(), derived_key);
        secure_storage_form.save(&config_json).await?;

        let mut forms_db = load_forms_db(secure_storage).await?;
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
        let object_id = &object_key.id();
        let meta: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&secure_storage.load().await?)?;
        let meta =
            meta.get(object_id)
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Configuration for {} not found",
                    object_id
                )))?;
        let password =
            meta.get("PASSWD")
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Configuration for {} not found",
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
        let form_id = &object_key.id();
        let mut forms_db = load_forms_db(secure_storage).await?;

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
        let form_id = &object_key.id();
        let mut forms_db = load_forms_db(secure_storage).await?;

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

async fn load_forms_db(
    secure_storage: &SecureStorage,
) -> SecureStringResult<HashMap<String, HashMap<String, String>>> {
    match secure_storage.load().await {
        Ok(passwords_json) => match serde_json::from_str(&passwords_json) {
            Ok(map) => Ok(map),
            Err(err) => Err(SecureStringError::SerdeError(format!(
                "Failed to parse forms_db: {:?}",
                err
            ))),
        },
        Err(_) => Ok(HashMap::new()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use wasm_bindgen_test::*;

    use super::*;
    use crate::stringvault::User;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_list() {
        let username = "test_user_configurations_test_list";
        let password = "password_for_list";

        // Create a user and get the secure_storage
        let user_result = User::new(username, password).await;
        assert!(user_result.is_ok());
        let user = user_result.unwrap();

        let configurations = Configurations {};

        let result = configurations.list(&user.secure_storage()).await;
        assert!(
            result.is_ok(),
            "Failed to list configurations: {:?}",
            result.err().unwrap()
        );

        let config_map = result.unwrap();
        assert_eq!(config_map.len(), 0);
        User::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_save_and_load() {
        let username = "test_user_configurations_save_load";
        let password = "password_for_save_load";

        // Create a user and get the secure_storage
        let user_result = User::new(username, password).await;
        assert!(user_result.is_ok());
        let user = user_result.unwrap();
        let mut secure_storage = user.secure_storage().clone();

        let configurations = Configurations {};

        let mut config = HashMap::new();
        config.insert("__NAME__".to_string(), "test_config".to_string());

        let object_key =
            ObjectKey::new("test_save_load", "test_id_save_load").unwrap();

        let save_result = configurations
            .save(&mut secure_storage, object_key.clone(), config.clone())
            .await;
        assert!(
            save_result.is_ok(),
            "Failed to save configuration: {:?}",
            save_result.err().unwrap()
        );

        let load_result = configurations
            .load(&secure_storage, object_key.clone())
            .await;
        assert!(
            load_result.is_ok(),
            "Failed to load configuration: {:?}",
            load_result.err().unwrap()
        );

        let loaded_config = load_result.unwrap();
        assert_eq!(
            loaded_config, config,
            "Loaded configuration did not match saved configuration"
        );
        User::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_add_and_delete() {
        let username = "test_user_configurations_add_and_delete";
        let password = "password_for_add_and_delete";

        // Create a unique user for this test and get the secure_storage
        let user_result = User::new(username, password).await;
        assert!(user_result.is_ok());
        let user = user_result.unwrap();
        let mut secure_storage = user.secure_storage().clone();

        let mut configurations = Configurations {};

        let mut config = HashMap::new();
        config.insert("__NAME__".to_string(), "test_config".to_string());

        let object_key =
            ObjectKey::new("test_add_delete", "test_id_add_delete").unwrap();

        // Test adding a configuration
        let add_result = configurations
            .save(&mut secure_storage, object_key.clone(), config.clone())
            .await;
        assert!(
            add_result.is_ok(),
            "Failed to add configuration: {:?}",
            add_result.err().unwrap()
        );

        let load_result = configurations
            .load(&secure_storage, object_key.clone())
            .await;
        assert!(
            load_result.is_ok(),
            "Failed to load configuration after add: {:?}",
            load_result.err().unwrap()
        );

        let loaded_config = load_result.unwrap();
        assert_eq!(
            loaded_config, config,
            "Loaded configuration did not match added configuration"
        );

        // Test deleting a configuration
        let delete_result = configurations
            .delete(&mut secure_storage, object_key.clone())
            .await;
        assert!(
            delete_result.is_ok(),
            "Failed to delete configuration: {:?}",
            delete_result.err().unwrap()
        );

        let load_result_after_delete = configurations
            .load(&secure_storage, object_key.clone())
            .await;
        assert!(
            load_result_after_delete.is_err(),
            "Successfully loaded configuration after delete"
        );
        User::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_delete_non_existent() {
        let username = "test_user_configurations_delete_non_existent";
        let password = "password_for_delete_non_existent";

        let user_result = User::new(username, password).await;
        assert!(user_result.is_ok());
        let user = user_result.unwrap();
        let mut secure_storage = user.secure_storage().clone();

        let mut configurations = Configurations {};
        let object_key = ObjectKey::new(
            "test_delete_non_existent",
            "test_id_delete_non_existent",
        )
        .unwrap();

        let delete_result = configurations
            .delete(&mut secure_storage, object_key.clone())
            .await;
        assert!(
            delete_result.is_err(),
            "Successfully deleted a non-existent configuration"
        );

        User::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_load_non_existent() {
        let username = "test_user_configurations_load_non_existent";
        let password = "password_for_load_non_existent";

        let user_result = User::new(username, password).await;
        assert!(user_result.is_ok());
        let user = user_result.unwrap();
        let secure_storage = user.secure_storage().clone();

        let configurations = Configurations {};
        let object_key = ObjectKey::new(
            "test_load_non_existent",
            "test_id_load_non_existent",
        )
        .unwrap();

        let load_result = configurations
            .load(&secure_storage, object_key.clone())
            .await;
        assert!(
            load_result.is_err(),
            "Successfully loaded a non-existent configuration"
        );

        User::reset(username).await.unwrap();
    }
}
