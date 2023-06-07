use std::collections::HashMap;

use log::info;
use serde_json;

use crate::crypto::derive_crypto_key;
use crate::utils::generate_password_base64;
use crate::{
    DocumentMetaData, ObjectKey, SecureStorage, SecureStringError,
    SecureStringResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Configurations {}

impl Configurations {
    pub async fn list(
        &self,
        secure_storage: &SecureStorage,
    ) -> SecureStringResult<Vec<DocumentMetaData>> {
        // Attempt to load the data.
        match secure_storage.load().await {
            // If successful, proceed with deserialization and transformation.
            Ok(data) => {
                // Load the stored form metadata.
                let forms_db: HashMap<String, HashMap<String, String>> =
                    serde_json::from_slice(&data)?;

                // Transform the metadata into the desired output format.
                let configurations = forms_db
                    .into_iter()
                    .map(|(id, tags)| {
                        // Construct FormData
                        DocumentMetaData::new_with_tags(&id, tags)
                    })
                    .collect();

                Ok(configurations)
            }
            // If loading fails with NoLocalStorageData, return an empty Vec.
            Err(SecureStringError::NoLocalStorageData) => Ok(Vec::new()),
            // For other errors, propagate the error.
            Err(err) => Err(err),
        }
    }

    pub async fn save(
        &self,
        secure_storage: &mut SecureStorage,
        meta_data: DocumentMetaData,
        document_content: &[u8],
    ) -> SecureStringResult<()> {
        let form_id = meta_data.id();

        info!("Saving configuration for {}", form_id);

        let password = generate_password_base64()?;
        let derived_key = derive_crypto_key(&password, &form_id).await?;

        // secure storage for the form
        let object_key = ObjectKey::new("", &form_id)?;
        let secure_storage_form = SecureStorage::new(object_key, derived_key);
        secure_storage_form.save(document_content).await?;

        let mut forms_db = load_forms_db(secure_storage).await?;
        let mut form_config = HashMap::new();

        // add PASSWD to the form metadata, required to load() the data
        form_config.insert("__PASSWD__".to_string(), password);

        // add every tag in form_data.tags() to form metadata
        // NOTE: we may have to limit this in size, or store (extra?) tags separately
        if let Some(tags) = meta_data.tags() {
            form_config.extend(tags);
        }
        forms_db.insert(form_id, form_config);
        secure_storage.save(&serde_json::to_vec(&forms_db)?).await
    }

    // Loads the configuration securely after decrypting it with a derived key
    pub async fn load(
        &self,
        secure_storage: &SecureStorage,
        form_id: &str,
    ) -> SecureStringResult<Vec<u8>> {
        let meta: HashMap<String, HashMap<String, String>> =
            serde_json::from_slice(&secure_storage.load().await?)?;
        let meta =
            meta.get(form_id)
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Configuration for {} not found",
                    form_id
                )))?;
        let password =
            meta.get("__PASSWD__")
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Configuration for {} not found",
                    form_id
                )))?;

        let object_key = ObjectKey::new("", &form_id).unwrap();
        let derived_key = derive_crypto_key(&password, &form_id).await?;
        let secure_storage_form = SecureStorage::new(object_key, derived_key);

        Ok(secure_storage_form.load().await?)
    }

    pub async fn add(
        &mut self,
        secure_storage: &SecureStorage,
        meta_data: DocumentMetaData,
    ) -> SecureStringResult<()> {
        // NOTE: this only updates vault-user's local copy of the forms_db
        //  it does not create a form itself like save() does.
        //  this function is created to quickly add/ delete in a list context
        //  (in this case a full save() would be too expensive, and unnecessary)
        //  may have to rename to make this more clear
        let mut forms_db = load_forms_db(secure_storage).await?;

        let form_id = meta_data.id();
        let form_config = forms_db
            .entry(form_id.to_string())
            .or_insert_with(HashMap::new);

        if let Some(tags) = meta_data.tags() {
            form_config.extend(tags);
        }
        //form_config.insert("NAME".to_string(), name);
        secure_storage.save(&serde_json::to_vec(&forms_db)?).await?;
        Ok(())
    }

    pub async fn delete(
        &mut self,
        secure_storage: &SecureStorage,
        form_id: &str,
    ) -> SecureStringResult<()> {
        let mut forms_db = load_forms_db(secure_storage).await?;

        // Remove the specific configuration
        if forms_db.remove(form_id).is_none() {
            return Err(SecureStringError::PasswordNotFound(format!(
                "Configuration for {} not found",
                form_id
            )));
        }

        // Save the updated form metadata back to the vault.
        secure_storage.save(&serde_json::to_vec(&forms_db)?).await?;

        Ok(())
    }
}

async fn load_forms_db(
    secure_storage: &SecureStorage,
) -> SecureStringResult<HashMap<String, HashMap<String, String>>> {
    match secure_storage.load().await {
        Ok(passwords_json) => match serde_json::from_slice(&passwords_json) {
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
    use crate::User;

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

        let form_id = "test_id_save_load";
        let meta_data = DocumentMetaData::new(form_id);
        let config_bytes = serde_json::to_vec(&config).unwrap();

        let save_result = configurations
            .save(&mut secure_storage, meta_data.clone(), &config_bytes)
            .await;
        assert!(
            save_result.is_ok(),
            "Failed to save configuration: {:?}",
            save_result.err().unwrap()
        );

        let load_result =
            configurations.load(&secure_storage, form_id).await;
        assert!(
            load_result.is_ok(),
            "Failed to load configuration: {:?}",
            load_result.err().unwrap()
        );

        let loaded_config_bytes = load_result.unwrap();
        let loaded_config: HashMap<String, String> =
            serde_json::from_slice(&loaded_config_bytes).unwrap();
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

        let form_id = "test_id_add_delete";
        let meta_data = DocumentMetaData::new(form_id);
        let config_bytes = serde_json::to_vec(&config).unwrap();
        // Test adding a configuration
        let add_result = configurations
            .save(&mut secure_storage, meta_data.clone(), &config_bytes)
            .await;
        assert!(
            add_result.is_ok(),
            "Failed to add configuration: {:?}",
            add_result.err().unwrap()
        );

        let load_result =
            configurations.load(&secure_storage, form_id).await;
        assert!(
            load_result.is_ok(),
            "Failed to load configuration after add: {:?}",
            load_result.err().unwrap()
        );

        let loaded_config_bytes = load_result.unwrap();
        let loaded_config: HashMap<String, String> =
            serde_json::from_slice(&loaded_config_bytes).unwrap();
        assert_eq!(
            loaded_config, config,
            "Loaded configuration did not match added configuration"
        );

        // Test deleting a configuration
        let delete_result = configurations
            .delete(&mut secure_storage, form_id)
            .await;
        assert!(
            delete_result.is_ok(),
            "Failed to delete configuration: {:?}",
            delete_result.err().unwrap()
        );

        let load_result_after_delete =
            configurations.load(&secure_storage, form_id).await;
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
        let form_name = format!(
            "{}:{}",
            "test_delete_non_existent", "test_id_delete_non_existent"
        );

        let delete_result =
            configurations.delete(&mut secure_storage, &form_name).await;
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
        let form_id = "test_id_load_non_existent";

        let load_result =
            configurations.load(&secure_storage, form_id).await;
        assert!(
            load_result.is_err(),
            "Successfully loaded a non-existent configuration"
        );

        User::reset(username).await.unwrap();
    }
}
