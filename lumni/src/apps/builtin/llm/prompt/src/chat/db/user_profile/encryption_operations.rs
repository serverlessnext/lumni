use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use dirs::home_dir;
use lumni::api::error::{ApplicationError, EncryptionError};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};

use super::{DatabaseOperationError, EncryptionHandler, UserProfileDbHandler};
use crate::external as lumni;

impl UserProfileDbHandler {
    pub fn encrypt_value(
        &self,
        content: &JsonValue,
    ) -> Result<JsonValue, ApplicationError> {
        let type_info = match content {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        };

        let content_string = match content {
            JsonValue::String(s) => s.clone(), // Use the string value directly
            _ => content.to_string(), // For other types, use JSON serialization
        };

        if let Some(ref encryption_handler) = self.encryption_handler {
            let (encrypted_content, encryption_key) = encryption_handler
                .encrypt_string(&content_string)
                .map_err(|e| {
                    ApplicationError::EncryptionError(
                        EncryptionError::EncryptionFailed(e.to_string()),
                    )
                })?;
            Ok(json!({
                "content": encrypted_content,
                "encryption_key": encryption_key,
                "type_info": type_info
            }))
        } else {
            Err(ApplicationError::EncryptionError(
                EncryptionError::InvalidKey(
                    "Encryption handler required to encrypt value".to_string(),
                ),
            ))
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
                    Some(JsonValue::String(type_info)),
                ) = (
                    obj.get("content"),
                    obj.get("encryption_key"),
                    obj.get("type_info"),
                ) {
                    let decrypted_string = encryption_handler
                        .decrypt_string(content, encrypted_key)
                        .map_err(|e| {
                            ApplicationError::EncryptionError(
                                EncryptionError::DecryptionFailed(
                                    e.to_string(),
                                ),
                            )
                        })?;

                    // Parse the decrypted string based on the type_info
                    let decrypted_value = match type_info.as_str() {
                        "null" => JsonValue::Null,
                        "boolean" => JsonValue::Bool(
                            decrypted_string.parse().map_err(|_| {
                                ApplicationError::EncryptionError(
                                    EncryptionError::DecryptionFailed(
                                        "Failed to parse boolean".to_string(),
                                    ),
                                )
                            })?,
                        ),
                        "number" => serde_json::from_str(&decrypted_string)
                            .map_err(|_| {
                                ApplicationError::EncryptionError(
                                    EncryptionError::DecryptionFailed(
                                        "Failed to parse number".to_string(),
                                    ),
                                )
                            })?,
                        "string" => JsonValue::String(decrypted_string), // Don't parse, use the string directly
                        "array" | "object" => serde_json::from_str(
                            &decrypted_string,
                        )
                        .map_err(|_| {
                            ApplicationError::EncryptionError(
                                EncryptionError::DecryptionFailed(
                                    "Failed to parse complex type".to_string(),
                                ),
                            )
                        })?,
                        _ => {
                            return Err(ApplicationError::EncryptionError(
                                EncryptionError::DecryptionFailed(
                                    "Unknown type".to_string(),
                                ),
                            ))
                        }
                    };

                    Ok(decrypted_value)
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
                    "Encryption handler required to decrypt value".to_string(),
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
                    "Encryption handler required to validate hash".to_string(),
                ),
            ))
        }
    }

    pub async fn get_or_create_encryption_key(
        &mut self,
    ) -> Result<i64, ApplicationError> {
        let has_encryption_handler = self.encryption_handler.is_some();
        let mut created_encryption_handler: Option<EncryptionHandler> = None;

        let mut db = self.db.lock().await;
        let key_id = db
            .process_queue_with_result(|tx| {
                if has_encryption_handler {
                    let key_path = self
                        .encryption_handler
                        .as_ref()
                        .unwrap()
                        .get_key_path();
                    let key_hash =
                        EncryptionHandler::get_private_key_hash(&key_path)?;
                    self.get_or_insert_encryption_key(tx, &key_path, &key_hash)
                } else {
                    let key_dir = home_dir()
                        .unwrap_or_default()
                        .join(".lumni")
                        .join("keys");
                    fs::create_dir_all(&key_dir)
                        .map_err(|e| ApplicationError::IOError(e))?;

                    let new_encryption_handler =
                        EncryptionHandler::generate_private_key(
                            &key_dir, 2048, None,
                        )
                        .map_err(|e| {
                            ApplicationError::EncryptionError(
                                EncryptionError::KeyGenerationFailed(
                                    e.to_string(),
                                ),
                            )
                        })?;
                    let key_path = new_encryption_handler.get_key_path();
                    let key_hash = new_encryption_handler.get_sha256_hash();
                    let key_id = self
                        .get_or_insert_encryption_key(tx, key_path, key_hash)?;
                    created_encryption_handler = Some(new_encryption_handler);
                    Ok(key_id)
                }
            })
            .map_err(|e| match e {
                DatabaseOperationError::SqliteError(sqlite_err) => {
                    ApplicationError::DatabaseError(sqlite_err.to_string())
                }
                DatabaseOperationError::ApplicationError(app_err) => app_err,
            })?;

        if let Some(new_handler) = created_encryption_handler {
            self.encryption_handler = Some(Arc::new(new_handler));
        }
        Ok(key_id)
    }
}
