use web_sys::CryptoKey;

use super::encryption::derive_crypto_key;
use super::error::{SecureStringError, SecureStringResult};
use super::storage::{create_storage_key, load_string, save_string};
use super::string_ops::generate_salt_base64;
use super::ObjectKey;

pub fn get_crypto_subtle(
) -> SecureStringResult<(web_sys::Window, web_sys::Crypto, web_sys::SubtleCrypto)>
{
    let window = web_sys::window().ok_or(SecureStringError::NoWindow)?;
    let crypto = window.crypto().map_err(|_| SecureStringError::NoCrypto)?;
    let subtle = crypto.subtle();
    Ok((window, crypto, subtle))
}

pub async fn load_or_save_salt(
    storage_key: &str,
) -> SecureStringResult<String> {
    match load_string(&storage_key).await {
        Some(salt) => Ok(salt),
        None => {
            let new_salt = generate_salt_base64()?;
            let save_result = save_string(&storage_key, &new_salt).await;
            if save_result.is_err() {
                // If saving the new salt fails, try to get the old salt again
                match load_string(&storage_key).await {
                    Some(salt) => Ok(salt),
                    None => Err(SecureStringError::SaltNotStored),
                }
            } else {
                Ok(new_salt)
            }
        }
    }
}

pub async fn derive_key_from_password(
    object_key: &ObjectKey,
    password: &str,
) -> SecureStringResult<CryptoKey> {
    let storage_key = create_storage_key(&object_key);
    let salt = load_or_save_salt(&storage_key).await?;
    derive_crypto_key(password, &salt).await
}

#[cfg(test)]
mod tests {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    use super::*;
    use crate::stringvault::storage::delete_string;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_get_crypto_subtle() {
        match get_crypto_subtle() {
            Ok((window, crypto, subtle)) => {
                // Assert that window, crypto and subtle are not null
                assert!(window.is_instance_of::<web_sys::Window>());
                assert!(crypto.is_instance_of::<web_sys::Crypto>());
                assert!(subtle.is_instance_of::<web_sys::SubtleCrypto>());
            }
            Err(e) => {
                // The test should not error out. If it does, fail the test.
                panic!("get_crypto_subtle returned an error: {:?}", e);
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_derive_key_same_password() {
        let object_key =
            ObjectKey::new("crypto_test1", "crypto_test_id1").unwrap();
        let password = "password";

        let result = derive_key_from_password(&object_key, password).await;
        assert!(result.is_ok());

        let result_repeat =
            derive_key_from_password(&object_key, password).await;
        assert!(result_repeat.is_ok());

        // Export keys
        let subtle = get_crypto_subtle().unwrap().2;
        let result_unwrapped = result.as_ref().unwrap();
        let exported_key_promise =
            subtle.export_key("raw", result_unwrapped).unwrap();
        let exported_key =
            wasm_bindgen_futures::JsFuture::from(exported_key_promise)
                .await
                .unwrap();

        let result_repeat_unwrapped = result_repeat.as_ref().unwrap();
        let exported_key_repeat_promise =
            subtle.export_key("raw", result_repeat_unwrapped).unwrap();
        let exported_key_repeat =
            wasm_bindgen_futures::JsFuture::from(exported_key_repeat_promise)
                .await
                .unwrap();

        let exported_key_bytes =
            js_sys::Uint8Array::new(&exported_key).to_vec();
        let exported_key_repeat_bytes =
            js_sys::Uint8Array::new(&exported_key_repeat).to_vec();
        assert_eq!(exported_key_bytes, exported_key_repeat_bytes);
    }

    #[wasm_bindgen_test]
    async fn test_derive_key_empty_password() {
        let object_key =
            ObjectKey::new("crypto_test2", "crypto_test_id2").unwrap();
        let password_empty = "";

        let result_empty =
            derive_key_from_password(&object_key, password_empty).await;
        assert!(result_empty.is_err());
    }

    #[wasm_bindgen_test]
    async fn test_derive_key_different_salt() {
        let object_key =
            ObjectKey::new("crypto_test3", "crypto_test_id3").unwrap();
        let password = "password";

        // Get the storage_key for saving the salt
        let storage_key = create_storage_key(&object_key);

        // First derive key with salt1
        let salt1 = "salt1";
        let save_result1 = save_string(&storage_key, salt1).await;
        assert!(save_result1.is_ok(), "Failed to save string for salt1.");
        let result_salt1 =
            derive_key_from_password(&object_key, password).await;
        assert!(result_salt1.is_ok());

        // Then derive key with salt2
        let salt2 = "salt2";
        let save_result2 = save_string(&storage_key, salt2).await;
        assert!(save_result2.is_ok(), "Failed to save string for salt2.");
        let result_salt2 =
            derive_key_from_password(&object_key, password).await;
        assert!(result_salt2.is_ok());

        // Finally, assert that the keys derived with different salts are different
        assert_ne!(
            result_salt1.as_ref().unwrap(),
            result_salt2.as_ref().unwrap()
        );
    }

    #[wasm_bindgen_test]
    async fn test_derive_key_same_salt_different_password() {
        let object_key =
            ObjectKey::new("crypto_test4", "crypto_test_id4").unwrap();

        // Derive key with password1
        let password1 = "password1";
        let result_password1 =
            derive_key_from_password(&object_key, password1).await;
        assert!(result_password1.is_ok());

        // Derive key with password2
        let password2 = "password2";
        let result_password2 =
            derive_key_from_password(&object_key, password2).await;
        assert!(result_password2.is_ok());

        // Finally, assert that the keys derived with different passwords are different
        assert_ne!(
            result_password1.as_ref().unwrap(),
            result_password2.as_ref().unwrap()
        );
    }

    #[wasm_bindgen_test]
    async fn test_load_or_save_salt() {
        let storage_key = "crypto_key_load_or_save_salt";

        // Cleanup before starting the test
        let _ = delete_string(&storage_key).await;

        // Test when there is no salt in the local storage
        let first_salt_result = load_or_save_salt(&storage_key).await;
        assert!(first_salt_result.is_ok());
        let first_salt = first_salt_result.unwrap();

        // The salt should now be in the local storage
        let second_salt_result = load_or_save_salt(&storage_key).await;
        assert!(second_salt_result.is_ok());
        let second_salt = second_salt_result.unwrap();

        // Both salts should be the same, because the second call should have retrieved the first salt from local storage
        assert_eq!(first_salt, second_salt);

        // Cleanup after the test
        let _ = delete_string(&storage_key).await;
    }
}
