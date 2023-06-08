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
pub struct DocumentMetaData {
    id: String,
    tags: Option<HashMap<String, String>>,
}

impl DocumentMetaData {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            tags: None,
        }
    }

    pub fn new_with_tags(id: &str, tags: HashMap<String, String>) -> Self {
        Self {
            id: id.to_string(),
            tags: Some(tags),
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn tags(&self) -> Option<HashMap<String, String>> {
        self.tags.clone()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct DocumentStore {
    secure_storage: SecureStorage,
    configurations: Configurations,
}

impl DocumentStore {
    pub fn new(secure_storage: SecureStorage) -> Self {
        Self {
            secure_storage,
            configurations: Configurations {},
        }
    }

    pub async fn list_configurations(
        &self,
    ) -> SecureStringResult<Vec<DocumentMetaData>> {
        self.configurations.list(&self.secure_storage).await
    }

    pub async fn save_configuration(
        &mut self,
        meta_data: DocumentMetaData,
        document_content: &[u8],
    ) -> SecureStringResult<()> {
        self.configurations
            .save(&mut self.secure_storage, meta_data, document_content)
            .await
    }

    pub async fn load_configuration(
        &self,
        form_id: &str,
    ) -> SecureStringResult<Vec<u8>> {
        self.configurations
            .load(&self.secure_storage, form_id)
            .await
    }

    pub async fn add_configuration(
        &mut self,
        meta_data: DocumentMetaData,
    ) -> SecureStringResult<()> {
        self.configurations
            .add(&self.secure_storage, meta_data)
            .await
    }

    pub async fn delete_configuration(
        &mut self,
        form_name: &str,
    ) -> SecureStringResult<()> {
        self.configurations
            .delete(&self.secure_storage, form_name)
            .await
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct LocalEncrypt {
    user: User,
}

impl LocalEncrypt {
    pub async fn new(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        let user = User::new(username, password).await?;
        Ok(Self { user })
    }

    pub fn user(&self) -> User {
        self.user.clone()
    }

    pub fn create_document_store(&self) -> DocumentStore {
        let secure_storage = self.user().secure_storage().clone();
        DocumentStore::new(secure_storage)
    }

    pub async fn create_or_validate(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        Ok(Self {
            user: User::create_or_validate(username, password).await?,
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
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use wasm_bindgen_test::*;

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_new() {
        let username = "test_local_encrypt_new";
        let password = "password_for_new";

        let local_encrypt_result = LocalEncrypt::new(username, password).await;
        assert!(
            local_encrypt_result.is_ok(),
            "Failed to create new LocalEncrypt"
        );

        LocalEncrypt::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_create_or_validate() {
        let username = "test_string_vault_create_or_validate";
        let password = "password_for_create_or_validate";

        let string_vault_result =
            LocalEncrypt::create_or_validate(username, password).await;
        assert!(
            string_vault_result.is_ok(),
            "Failed to create or validate StringVault"
        );

        LocalEncrypt::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_change_password() {
        let username = "test_string_vault_change_password";
        let old_password = "password_for_change_password";
        let new_password = "new_password_for_change_password";

        LocalEncrypt::new(username, old_password).await.unwrap();
        let change_password_result =
            LocalEncrypt::change_password(username, old_password, new_password)
                .await;
        assert!(change_password_result.is_ok(), "Failed to change password");

        LocalEncrypt::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_user_exists() {
        let username = "test_string_vault_exists";
        let password = "password_for_exists";

        // Assert Vault doesn't exist initially
        assert_eq!(LocalEncrypt::user_exists(username).await, false);

        // Create the Vault
        LocalEncrypt::create_or_validate(username, password)
            .await
            .unwrap();

        // Assert Vault now exists
        assert_eq!(LocalEncrypt::user_exists(username).await, true);

        // Reset the Vault
        LocalEncrypt::reset(username).await.unwrap();

        // Assert StringVault doesn't exist now
        assert_eq!(LocalEncrypt::user_exists(username).await, false);
        LocalEncrypt::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_validate_password() {
        let username = "test_string_vault_validate_password";
        let password = "password_for_validate_password";

        LocalEncrypt::create_or_validate(username, password)
            .await
            .unwrap();
        let validate_password_result =
            LocalEncrypt::validate_password(username, password).await;
        assert!(
            validate_password_result.is_ok()
                && validate_password_result.unwrap(),
            "Failed to validate password"
        );

        LocalEncrypt::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_list_configurations() {
        let username = "test_string_vault_list_configurations";
        let password = "password_for_list";

        let local_encrypt =
            LocalEncrypt::create_or_validate(username, password)
                .await
                .unwrap();

        let mut config = HashMap::new();
        config
            .insert("some random value".to_string(), "test_config".to_string());

        let form_id = "test_id_list";
        let meta_data = DocumentMetaData::new(form_id);

        let config_bytes = serde_json::to_vec(&config).unwrap();
        let mut document_store = local_encrypt.create_document_store();
        let save_result = document_store
            .save_configuration(meta_data, &config_bytes)
            .await;
        assert!(
            save_result.is_ok(),
            "Failed to save secure configuration: {:?}",
            save_result.err().unwrap()
        );

        let list_result = document_store.list_configurations().await;
        assert!(
            list_result.is_ok(),
            "Failed to list configurations: {:?}",
            list_result.err().unwrap()
        );

        let listed_configurations = list_result.unwrap();
        assert!(listed_configurations
            .iter()
            .any(|form_data| form_data.id() == form_id));

        LocalEncrypt::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_add_and_delete_configuration() {
        let username = "test_string_vault_add_delete";
        let password = "password_for_add_delete";

        let local_encrypt =
            LocalEncrypt::new(username, password).await.unwrap();
        let mut document_store = local_encrypt.create_document_store();

        let form_id = "test_id_add_delete";
        let meta_data = DocumentMetaData::new(&form_id);

        // Add a configuration with a given name
        let add_result = document_store.add_configuration(meta_data).await;
        assert!(
            add_result.is_ok(),
            "Failed to add configuration: {:?}",
            add_result.err().unwrap()
        );

        // Delete the configuration
        let delete_result = document_store.delete_configuration(&form_id).await;
        assert!(
            delete_result.is_ok(),
            "Failed to delete configuration: {:?}",
            delete_result.err().unwrap()
        );

        // Try to delete again, it should fail since the configuration no longer exists
        let delete_again_result =
            document_store.delete_configuration(&form_id).await;
        assert!(
            delete_again_result.is_err(),
            "Successfully deleted non-existent configuration"
        );

        LocalEncrypt::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_save_and_load_configuration() {
        let username = "test_local_encrypt_save_load";
        let password = "password_for_save_load";

        let local_encrypt =
            LocalEncrypt::create_or_validate(username, password)
                .await
                .unwrap();

        let mut document_store = local_encrypt.create_document_store();

        let mut config = HashMap::new();
        config
            .insert("some random value".to_string(), "test_config".to_string());

        let form_id = username;
        let meta_data = DocumentMetaData::new(form_id);

        let config_bytes = serde_json::to_vec(&config).unwrap();
        let save_result = document_store
            .save_configuration(meta_data.clone(), &config_bytes)
            .await;
        assert!(
            save_result.is_ok(),
            "Failed to save secure configuration: {:?}",
            save_result.err().unwrap()
        );

        let load_result = document_store.load_configuration(form_id).await;
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

        LocalEncrypt::reset(username).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_delete_configuration() {
        let username = "test_string_vault_delete";
        let password = "password_for_delete";

        let local_encrypt =
            LocalEncrypt::create_or_validate(username, password)
                .await
                .unwrap();

        let mut config = HashMap::new();
        config
            .insert("some random value".to_string(), "test_config".to_string());

        let form_id = username;
        let meta_data = DocumentMetaData::new(form_id);

        let config_bytes = serde_json::to_vec(&config).unwrap();
        let mut document_store = local_encrypt.create_document_store();
        let save_result = document_store
            .save_configuration(meta_data.clone(), &config_bytes)
            .await;
        assert!(
            save_result.is_ok(),
            "Failed to save data: {:?}",
            save_result.err().unwrap()
        );

        let delete_result = document_store.delete_configuration(form_id).await;
        assert!(
            delete_result.is_ok(),
            "Failed to delete data: {:?}",
            delete_result.err().unwrap()
        );

        let load_result = document_store.load_configuration(form_id).await;
        assert!(
            load_result.is_err(),
            "Successfully loaded data after deletion"
        );

        LocalEncrypt::reset(username).await.unwrap();
    }
}
