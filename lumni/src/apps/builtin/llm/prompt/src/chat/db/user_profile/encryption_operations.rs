use std::fs;
use std::path::PathBuf;

use dirs::home_dir;
use lumni::api::error::{ApplicationError, EncryptionError};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};

use super::{EncryptionHandler, UserProfileDbHandler};
use crate::external as lumni;

impl UserProfileDbHandler {
    pub fn encrypt_value(
        &self,
        content: &str,
    ) -> Result<JsonValue, ApplicationError> {
        if let Some(ref encryption_handler) = self.encryption_handler {
            let (encrypted_content, encryption_key) =
                encryption_handler.encrypt_string(content).map_err(|e| {
                    ApplicationError::EncryptionError(
                        EncryptionError::EncryptionFailed(e.to_string()),
                    )
                })?;

            Ok(json!({
                "content": encrypted_content,
                "encryption_key": encryption_key,
            }))
        } else {
            Ok(JsonValue::String(content.to_string()))
        }
    }

    pub fn decrypt_value(
        &self,
        value: &JsonValue,
    ) -> Result<JsonValue, ApplicationError> {
        if let Some(ref encryption_handler) = self.encryption_handler {
            if let Some(obj) = value.as_object() {
                if let (
                    Some(JsonValue::String(content)),
                    Some(JsonValue::String(encrypted_key)),
                ) = (obj.get("content"), obj.get("encryption_key"))
                {
                    if encrypted_key.is_empty() {
                        return Ok(JsonValue::String(content.clone()));
                    }

                    encryption_handler
                        .decrypt_string(content, encrypted_key)
                        .map(JsonValue::String)
                        .map_err(|e| {
                            eprintln!("Decryption error: {:?}", e);
                            eprintln!(
                                "Content length: {}, Key length: {}",
                                content.len(),
                                encrypted_key.len()
                            );
                            ApplicationError::EncryptionError(
                                EncryptionError::DecryptionFailed(
                                    e.to_string(),
                                ),
                            )
                        })
                } else {
                    Err(ApplicationError::EncryptionError(
                        EncryptionError::InvalidKey(
                            "Invalid encrypted value format".to_string(),
                        ),
                    ))
                }
            } else {
                Err(ApplicationError::EncryptionError(
                    EncryptionError::InvalidKey(
                        "Value is not an object".to_string(),
                    ),
                ))
            }
        } else {
            Err(ApplicationError::EncryptionError(
                EncryptionError::InvalidKey(
                    "No encryption handler available".to_string(),
                ),
            ))
        }
    }

    pub fn verify_encryption_key_hash(
        &self,
        stored_hash: &str,
    ) -> Result<(), ApplicationError> {
        let current_hash = self.calculate_current_key_hash()?;
        if current_hash != stored_hash {
            return Err(ApplicationError::EncryptionError(
                EncryptionError::InvalidKey(
                    "Encryption key hash mismatch".to_string(),
                ),
            ));
        }
        Ok(())
    }

    fn calculate_current_key_hash(&self) -> Result<String, ApplicationError> {
        if let Some(ref encryption_handler) = self.encryption_handler {
            let key_data =
                encryption_handler.get_private_key_pem().map_err(|e| {
                    ApplicationError::EncryptionError(
                        EncryptionError::InvalidKey(e.to_string()),
                    )
                })?;
            let mut hasher = Sha256::new();
            hasher.update(key_data.as_bytes());
            Ok(format!("{:x}", hasher.finalize()))
        } else {
            Err(ApplicationError::EncryptionError(
                EncryptionError::InvalidKey(
                    "No encryption handler available".to_string(),
                ),
            ))
        }
    }
    pub fn generate_encryption_key(
        profile_name: &str,
    ) -> Result<(EncryptionHandler, PathBuf, String), ApplicationError> {
        let key_dir =
            home_dir().unwrap_or_default().join(".lumni").join("keys");
        fs::create_dir_all(&key_dir)
            .map_err(|e| ApplicationError::IOError(e))?;
        let key_path = key_dir.join(format!("{}_key.pem", profile_name));

        let encryption_handler =
            EncryptionHandler::generate_private_key(&key_path, 2048, None)
                .map_err(|e| {
                    ApplicationError::EncryptionError(
                        EncryptionError::KeyGenerationFailed(e.to_string()),
                    )
                })?;
        let key_hash = EncryptionHandler::get_private_key_hash(&key_path)
            .map_err(|e| {
                ApplicationError::EncryptionError(EncryptionError::InvalidKey(
                    e.to_string(),
                ))
            })?;

        Ok((encryption_handler, key_path, key_hash))
    }
}
