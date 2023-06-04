pub mod configurations;
mod convert_types;
mod crypto;
pub mod encryption;
pub mod error;
mod key_generation;
pub mod secure_storage;
mod storage;
pub mod string_ops;
pub mod user;

use std::collections::HashMap;

pub use configurations::Configurations;
pub use encryption::{decrypt, derive_crypto_key, encrypt, hash_username};
pub use error::{SecureStringError, SecureStringResult};
pub use secure_storage::SecureStorage;
pub use string_ops::generate_password_base64;
pub use user::User;

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

    pub async fn save_secure_configuration(
        &mut self,
        object_key: ObjectKey,
        config: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        self.configurations
            .save(&mut self.user.secure_storage().clone(), object_key, config)
            .await
    }

    pub async fn load_secure_configuration(
        &self,
        object_key: ObjectKey,
    ) -> SecureStringResult<HashMap<String, String>> {
        self.configurations
            .load(&self.user.secure_storage(), object_key)
            .await
    }

    pub async fn add_configuration(
        &mut self,
        object_key: ObjectKey,
        name: String,
    ) -> SecureStringResult<()> {
        self.configurations
            .add(&mut self.user.secure_storage(), object_key, name)
            .await
    }

    pub async fn delete_configuration(
        &mut self,
        object_key: ObjectKey,
    ) -> SecureStringResult<()> {
        self.configurations
            .delete(&mut self.user.secure_storage(), object_key)
            .await
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ObjectKey {
    tag: String,
    id: String,
}

impl ObjectKey {
    pub fn new(tag: &str, id: &str) -> Result<Self, SecureStringError> {
        if tag.is_empty() || id.is_empty() {
            return Err(SecureStringError::InvalidArgument(String::from(
                "Tag and ID must not be empty",
            )));
        }

        Ok(Self {
            tag: tag.to_string(),
            id: id.to_string(),
        })
    }

    pub fn new_with_form_tag(id: &str) -> Result<Self, SecureStringError> {
        if id.is_empty() {
            return Err(SecureStringError::InvalidArgument(String::from(
                "ID must not be empty",
            )));
        }

        Ok(Self {
            tag: "FORM".to_string(),
            id: id.to_string(),
        })
    }

    pub fn tag(&self) -> String {
        self.tag.clone()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;

    #[wasm_bindgen_test]
    fn test_object_key_new() {
        let object_key = ObjectKey::new("test", "test_id").unwrap();
        assert_eq!(object_key.tag(), "test");
        assert_eq!(object_key.id(), "test_id");
    }

    #[wasm_bindgen_test]
    fn test_object_key_new_with_form_tag() {
        let object_key = ObjectKey::new_with_form_tag("test_id").unwrap();
        assert_eq!(object_key.tag(), "FORM");
        assert_eq!(object_key.id(), "test_id");
    }

    #[wasm_bindgen_test]
    async fn test_invalid_object_key_creation() {
        // Check that ObjectKey::new returns an error when given an empty id
        let object_key_empty_id = ObjectKey::new("test_tag", "");
        assert!(
            object_key_empty_id.is_err(),
            "Successfully created ObjectKey with empty id"
        );

        // Check that ObjectKey::new returns an error when given an empty tag
        let object_key_empty_tag = ObjectKey::new("", "test_id");
        assert!(
            object_key_empty_tag.is_err(),
            "Successfully created ObjectKey with empty tag"
        );
    }
}
