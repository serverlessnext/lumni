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

        let object_key_user = ObjectKey {
            tag: "USER".to_string(),
            id: hashed_username.to_string(),
        };
        let crypto_key =
            derive_key_from_password(&object_key_user, password).await?;

        let object_key_crypto = ObjectKey {
            tag: object_key_user.id(),
            id: "self".to_string(),
        };
        Ok(Self {
            secure_storage: SecureStorage::new(object_key_crypto, crypto_key),
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

    pub async fn create(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        // note this will overwrite any existing user
        User::reset(username).await?;
        let user = User::new(username, password).await?;
        user.secure_storage.save("").await?;
        Ok(user)
    }

    pub async fn create_or_validate(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        if !User::exists(username).await {
            return User::create(username, password).await;
        }

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
        match User::create_or_validate(username, password).await {
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

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_new() {
        let username = "username";
        let password = "password";

        let user_result = User::new(username, password).await;
        assert!(user_result.is_ok());

        let user = user_result.unwrap();
        assert_eq!(
            user.secure_storage.object_key().tag(),
            hash_username(username)
        );
        assert_eq!(user.secure_storage.object_key().id(), "self");
    }

    #[wasm_bindgen_test]
    async fn test_create_or_validate_new_user() {
        let username = "new_username";
        let password = "new_password";

        // Resetting a non-existing user should not return an error
        assert!(User::reset(username).await.is_ok());

        let user_result = User::create_or_validate(username, password).await;
        assert!(user_result.is_ok());

        let user = user_result.unwrap();
        assert_eq!(
            user.secure_storage.object_key().tag(),
            hash_username(username)
        );
        assert_eq!(user.secure_storage.object_key().id(), "self");
    }

    #[wasm_bindgen_test]
    async fn test_create_or_validate_existing_user_wrong_password() {
        let username = "existing_username";
        let password = "correct_password";
        let wrong_password = "wrong_password";

        // Create the user
        User::create(username, password).await.unwrap();

        // Now try to validate the user with wrong password
        let user_result =
            User::create_or_validate(username, wrong_password).await;
        assert!(user_result.is_err());
        assert_eq!(
            user_result.unwrap_err(),
            SecureStringError::DecryptError(
                "Please ensure the password is correct.".to_owned()
            )
        );
    }

    #[wasm_bindgen_test]
    async fn test_create_or_validate_existing_user_correct_password() {
        let username = "existing_username_2";
        let password = "correct_password_2";

        // Create the user
        User::create(username, password).await.unwrap();

        // Now try to validate the user with correct password
        let user_result = User::create_or_validate(username, password).await;
        assert!(user_result.is_ok());

        let user = user_result.unwrap();
        assert_eq!(
            user.secure_storage.object_key().tag(),
            hash_username(username)
        );
        assert_eq!(user.secure_storage.object_key().id(), "self");
    }

    #[wasm_bindgen_test]
    async fn test_exists() {
        let username = "username_for_exists_test";
        let password = "password_for_exists_test";

        // Assert user doesn't exist initially
        assert_eq!(User::exists(username).await, false);

        // Create the user
        User::create(username, password).await.unwrap();

        // Assert user now exists
        assert_eq!(User::exists(username).await, true);
    }

    #[wasm_bindgen_test]
    async fn test_reset() {
        let username = "username_for_reset_test";
        let password = "password_for_reset_test";

        // Create the user
        User::create(username, password).await.unwrap();

        // Assert user now exists
        assert_eq!(User::exists(username).await, true);

        // Reset the user
        User::reset(username).await.unwrap();

        // Assert user doesn't exist now
        assert_eq!(User::exists(username).await, false);
    }
}
