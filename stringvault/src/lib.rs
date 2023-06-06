pub(crate) mod common;
mod configurations;
pub(crate) mod crypto;
pub(crate) mod storage;
mod user;
pub(crate) mod utils;

use std::collections::HashMap;

pub use common::{ObjectKey, SecureStringError, SecureStringResult};
use configurations::Configurations;
use storage::SecureStorage;
use user::User;

#[derive(Clone, PartialEq, Debug)]
pub struct FormMetaData {
    id: String,
    name: String,
}

impl FormMetaData {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct StringVault {
    user: User,
    configurations: Configurations,
}

impl StringVault {
    pub async fn new(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        Ok(Self {
            user: User::new(username, password).await?,
            configurations: Configurations {},
        })
    }

    pub async fn create_or_validate(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        Ok(Self {
            user: User::create_or_validate(username, password).await?,
            configurations: Configurations {},
        })
    }

    pub async fn user_exists(username: &str) -> bool {
        User::exists(username).await
    }

    pub async fn validate_password(
        username: &str,
        password: &str,
    ) -> Result<bool, SecureStringError> {
        User::validate_password(username, password).await
    }

    pub async fn change_password(
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> SecureStringResult<()> {
        User::change_password(username, old_password, new_password).await
    }

    pub async fn reset(username: &str) -> SecureStringResult<()> {
        User::reset(username).await
    }

    // Configurations related functions
    pub async fn list_configurations(
        &self,
    ) -> SecureStringResult<HashMap<String, String>> {
        self.configurations.list(&self.user.secure_storage()).await
    }

    pub async fn save_configuration(
        &mut self,
        form_meta: FormMetaData,
        data: &[u8],
    ) -> SecureStringResult<()> {
        // TODO: change form_meta input to be &[u8], which we convert (back) into FormMetaData
        // while this may seem un-necessary overhead, this will allow compatibility with other languages
        // given meta data is tiny performance penalty should be negligible
        self.configurations
            .save(&mut self.user.secure_storage().clone(), form_meta, data)
            .await
    }


    pub async fn load_configuration(
        &self,
        form_name: &str,
    ) -> SecureStringResult<Vec<u8>> {
        self.configurations
            .load(&self.user.secure_storage(), form_name)
            .await
    }

    pub async fn add_configuration(
        &mut self,
        form_name: &str,
        name: String,
    ) -> SecureStringResult<()> {
        self.configurations
            .add(&mut self.user.secure_storage(), form_name, name)
            .await
    }

    pub async fn delete_configuration(
        &mut self,
        form_name: &str,
    ) -> SecureStringResult<()> {
        self.configurations
            .delete(&mut self.user.secure_storage(), form_name)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use wasm_bindgen_test::*;

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_new() {
        let username = "test_string_vault_new";
        let password = "password_for_new";

        let string_vault_result = StringVault::new(username, password).await;
        assert!(
            string_vault_result.is_ok(),
            "Failed to create new StringVault"
        );

        StringVault::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_create_or_validate() {
        let username = "test_string_vault_create_or_validate";
        let password = "password_for_create_or_validate";

        let string_vault_result =
            StringVault::create_or_validate(username, password).await;
        assert!(
            string_vault_result.is_ok(),
            "Failed to create or validate StringVault"
        );

        StringVault::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_change_password() {
        let username = "test_string_vault_change_password";
        let old_password = "password_for_change_password";
        let new_password = "new_password_for_change_password";

        StringVault::new(username, old_password).await.unwrap();
        let change_password_result =
            StringVault::change_password(username, old_password, new_password)
                .await;
        assert!(change_password_result.is_ok(), "Failed to change password");

        StringVault::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_user_exists() {
        let username = "test_string_vault_exists";
        let password = "password_for_exists";

        // Assert StringVault doesn't exist initially
        assert_eq!(StringVault::user_exists(username).await, false);

        // Create the StringVault
        StringVault::create_or_validate(username, password)
            .await
            .unwrap();

        // Assert StringVault now exists
        assert_eq!(StringVault::user_exists(username).await, true);

        // Reset the StringVault
        StringVault::reset(username).await.unwrap();

        // Assert StringVault doesn't exist now
        assert_eq!(StringVault::user_exists(username).await, false);
        StringVault::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_validate_password() {
        let username = "test_string_vault_validate_password";
        let password = "password_for_validate_password";

        StringVault::create_or_validate(username, password)
            .await
            .unwrap();
        let validate_password_result =
            StringVault::validate_password(username, password).await;
        assert!(
            validate_password_result.is_ok()
                && validate_password_result.unwrap(),
            "Failed to validate password"
        );

        StringVault::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_list_configurations() {
        let username = "test_string_vault_list_configurations";
        let password = "password_for_list";

        let mut string_vault =
            StringVault::create_or_validate(username, password)
                .await
                .unwrap();

        let mut config = HashMap::new();
        config.insert("some random value".to_string(), "test_config".to_string());

        let form_meta = FormMetaData::new("test_id_list".to_string(), username.to_string());

        let config_bytes = serde_json::to_vec(&config).unwrap();
        let save_result = string_vault
            .save_configuration(form_meta.clone(), &config_bytes)
            .await;
        assert!(
            save_result.is_ok(),
            "Failed to save secure configuration: {:?}",
            save_result.err().unwrap()
        );

        let list_result = string_vault.list_configurations().await;
        assert!(
            list_result.is_ok(),
            "Failed to list configurations: {:?}",
            list_result.err().unwrap()
        );

        let listed_configurations = list_result.unwrap();
        assert!(
            listed_configurations.contains_key(form_meta.id()),
            "Listed configurations did not contain saved configuration"
        );

        StringVault::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_add_and_delete_configuration() {
        let username = "test_string_vault_add_delete";
        let password = "password_for_add_delete";

        let mut string_vault =
            StringVault::new(username, password).await.unwrap();

        let form_name =
            format!("{}:{}", "test_add_delete", "test_id_add_delete");

        // Add a configuration with a given name
        let add_result = string_vault
            .add_configuration(&form_name, "test_config".to_string())
            .await;
        assert!(
            add_result.is_ok(),
            "Failed to add configuration: {:?}",
            add_result.err().unwrap()
        );

        // Delete the configuration
        let delete_result = string_vault.delete_configuration(&form_name).await;
        assert!(
            delete_result.is_ok(),
            "Failed to delete configuration: {:?}",
            delete_result.err().unwrap()
        );

        // Try to delete again, it should fail since the configuration no longer exists
        let delete_again_result =
            string_vault.delete_configuration(&form_name).await;
        assert!(
            delete_again_result.is_err(),
            "Successfully deleted non-existent configuration"
        );

        StringVault::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_save_and_load_configuration() {
        let username = "test_string_vault_save_load";
        let password = "password_for_save_load";

        let mut string_vault =
            StringVault::create_or_validate(username, password)
                .await
                .unwrap();

        let mut config = HashMap::new();
        config.insert("some random value".to_string(), "test_config".to_string());

        let form_meta = FormMetaData::new(username.to_string(), "test_id_save_load".to_string());
        let config_bytes = serde_json::to_vec(&config).unwrap();
        let save_result = string_vault
            .save_configuration(form_meta.clone(), &config_bytes)
            .await;
        assert!(
            save_result.is_ok(),
            "Failed to save secure configuration: {:?}",
            save_result.err().unwrap()
        );

        let load_result = string_vault.load_configuration(form_meta.id()).await;
        assert!(
            load_result.is_ok(),
            "Failed to load secure configuration: {:?}",
            load_result.err().unwrap()
        );

        let loaded_config_bytes = load_result.unwrap();
        let loaded_config: HashMap<String, String> =
            serde_json::from_slice(&loaded_config_bytes).unwrap();
        assert_eq!(
            loaded_config, config,
            "Loaded secure configuration did not match saved configuration"
        );

        StringVault::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_delete_configuration() {
        let username = "test_string_vault_delete";
        let password = "password_for_delete";

        let mut string_vault =
            StringVault::create_or_validate(username, password)
                .await
                .unwrap();

        let mut config = HashMap::new();
        config.insert("some random value".to_string(), "test_config".to_string());

        let form_meta = FormMetaData::new(username.to_string(), "test_id_delete".to_string());
        let config_bytes = serde_json::to_vec(&config).unwrap();
        let save_result = string_vault
            .save_configuration(form_meta.clone(), &config_bytes)
            .await;
        assert!(
            save_result.is_ok(),
            "Failed to save data: {:?}",
            save_result.err().unwrap()
        );

        let delete_result = string_vault.delete_configuration(form_meta.id()).await;
        assert!(
            delete_result.is_ok(),
            "Failed to delete data: {:?}",
            delete_result.err().unwrap()
        );

        let load_result = string_vault.load_configuration(form_meta.id()).await;
        assert!(
            load_result.is_err(),
            "Successfully loaded data after deletion"
        );

        StringVault::reset(username).await.unwrap();
    }
}
