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
pub struct ObjectKey {
    pub tag: String,
    pub id: String,
}

impl ObjectKey {
    pub fn new_with_form_tag(id: String) -> Self {
        Self {
            tag: "FORM".to_string(),
            id,
        }
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

    pub async fn user_exists(username: &str) -> bool {
        User::exists(username).await
    }

    pub async fn new_and_validate(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        Ok(Self {
            user: User::new_and_validate(username, password).await?,
            configurations: Configurations {},
        })
    }

    pub async fn new_and_create(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        Ok(Self {
            user: User::new_and_create(username, password).await?,
            configurations: Configurations {},
        })
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
