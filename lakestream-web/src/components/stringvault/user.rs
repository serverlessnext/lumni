use super::crypto::derive_key_from_password;
use super::encryption::hash_username;
use super::error::SecureStringError;
use super::secure_storage::SecureStorage;
use super::{ObjectKey, SecureStringResult};

#[derive(Clone, PartialEq, Debug)]
pub struct User {
    secure_storage: SecureStorage,
}

impl User {
    pub async fn new(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        let hashed_username = hash_username(username);
        let crypto_key =
            derive_key_from_password(&hashed_username, password).await?;
        let object_key = ObjectKey {
            tag: hashed_username.clone(),
            id: "self".to_string(),
        };

        Ok(Self {
            secure_storage: SecureStorage::new(object_key, crypto_key),
        })
    }

    pub fn secure_storage(&self) -> &SecureStorage {
        &self.secure_storage
    }

    pub async fn exists(username: &str) -> bool {
        let hashed_username = hash_username(username);
        let object_key = ObjectKey {
            tag: hashed_username.clone(),
            id: "self".to_string(),
        };
        SecureStorage::exists(object_key).await
    }

    pub async fn new_and_validate(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        let user = User::new(username, password).await?;

        // Try to load the passwords to validate the password
        match user.secure_storage.load().await {
            Ok(_) => Ok(user),
            Err(err) => match err {
                SecureStringError::NoLocalStorageData => {
                    // user is not yet created
                    Ok(user)
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

    pub async fn validate_password(
        username: &str,
        password: &str,
    ) -> Result<bool, SecureStringError> {
        match User::new_and_validate(username, password).await {
            Ok(_) => Ok(true), /* Password is valid if new_and_validate doesn't return an error */
            Err(SecureStringError::DecryptError(_)) => Ok(false), /* DecryptError indicates an invalid password */
            Err(err) => Err(err), // Propagate any other errors
        }
    }

    #[allow(unused)]
    pub async fn change_password(
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> SecureStringResult<()> {
        // TODO: implement
        Ok(())
    }

    pub async fn new_and_create(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        // TODO: check if user exists
        User::reset(username).await?;
        let user = User::new(username, password).await?;

        // Try to load the passwords to validate the password
        match user.secure_storage.load().await {
            Ok(_) => Ok(user),
            Err(err) => match err {
                SecureStringError::NoLocalStorageData => {
                    // user is not yet created
                    Ok(user)
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

    pub async fn reset(username: &str) -> SecureStringResult<()> {
        let hashed_username = hash_username(username);
        let object_key = ObjectKey {
            tag: hashed_username.clone(),
            id: "self".to_string(),
        };

        let secure_storage = SecureStorage::for_deletion(object_key);
        secure_storage.delete().await?;

        Ok(())
    }
}
