use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use dirs::home_dir;
use lumni::api::error::{ApplicationError, EncryptionError};
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};

use super::{DatabaseOperationError, EncryptionHandler, UserProfileDbHandler};
use crate::external as lumni;

const DEFAULT_KEY_NAME: &str = "lumni_default_privkey";

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

    pub fn set_encryption_handler(
        &mut self,
        encryption_handler: Arc<EncryptionHandler>,
    ) -> Result<(), ApplicationError> {
        // If profile is not yet set, return error as we need to know the profile to validate against existing encryption handler
        let profile = self.profile.as_ref().ok_or_else(|| {
            ApplicationError::InvalidInput(
                "UserProfile must be defined before setting encryption handler"
                    .to_string(),
            )
        })?;
        // Check if the profile exists in the database and compare encryption handlers
        let db = self.db.clone();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut db = db.lock().await;
                db.process_queue_with_result(|tx| {
                    let existing_key: Option<(String, String)> = tx
                        .query_row(
                            "SELECT encryption_keys.file_path, \
                             encryption_keys.sha256_hash
                             FROM configuration
                             JOIN encryption_keys ON \
                             configuration.encryption_key_id = \
                             encryption_keys.id
                             WHERE configuration.name = ? AND \
                             configuration.section = 'profile'",
                            params![profile.name],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .optional()
                        .map_err(DatabaseOperationError::SqliteError)?;
                    if let Some((_, existing_hash)) = existing_key {
                        let new_path = encryption_handler.get_key_path();
                        let new_hash =
                            EncryptionHandler::get_private_key_hash(&new_path)?;
                        if existing_hash != new_hash {
                            return Err(
                                DatabaseOperationError::ApplicationError(
                                    ApplicationError::InvalidInput(
                                        "New encryption handler does not \
                                         match the existing one for this \
                                         profile"
                                            .to_string(),
                                    ),
                                ),
                            );
                        }
                    }
                    Ok(())
                })
            })
        });
        result.map_err(|e| match e {
            DatabaseOperationError::SqliteError(sqlite_err) => {
                ApplicationError::DatabaseError(sqlite_err.to_string())
            }
            DatabaseOperationError::ApplicationError(app_err) => app_err,
        })?;
        // If we've made it this far, either the profile doesn't exist yet or the encryption handler matches
        self.encryption_handler = Some(encryption_handler);
        Ok(())
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
                    // no key in database -- create a new (default) key, or load one from disk
                    let key_dir = home_dir()
                        .unwrap_or_default()
                        .join(".lumni")
                        .join("keys");
                    fs::create_dir_all(&key_dir)
                        .map_err(|e| ApplicationError::IOError(e))?;

                    let new_encryption_handler =
                        EncryptionHandler::load_or_generate_private_key(
                            &key_dir,
                            2048,
                            DEFAULT_KEY_NAME,
                            None,
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

    pub async fn register_encryption_key(
        &self,
        name: &str,
        file_path: &PathBuf,
    ) -> Result<(), ApplicationError> {
        let hash = EncryptionHandler::get_private_key_hash(file_path)?;
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "INSERT INTO encryption_keys (name, file_path, sha256_hash) \
                 VALUES (?, ?, ?)",
                params![name, file_path.to_str().unwrap(), hash],
            )
            .map_err(|e| DatabaseOperationError::SqliteError(e))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_encryption_key(
        &self,
        name: &str,
    ) -> Result<(String, String), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT file_path, sha256_hash FROM encryption_keys WHERE \
                 name = ?",
                params![name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| DatabaseOperationError::SqliteError(e))
        })
        .map_err(ApplicationError::from)
    }

    pub async fn remove_encryption_key(
        &self,
        name: &str,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "DELETE FROM encryption_keys WHERE name = ?",
                params![name],
            )
            .map_err(|e| DatabaseOperationError::SqliteError(e))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    pub async fn list_encryption_keys(
        &self,
        key_type: Option<&str>,
    ) -> Result<Vec<String>, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let query = match key_type {
                Some(_) => {
                    "SELECT name FROM encryption_keys WHERE key_type = ?"
                }
                None => "SELECT name FROM encryption_keys",
            };

            let mut stmt = tx
                .prepare(query)
                .map_err(|e| DatabaseOperationError::SqliteError(e))?;

            let row_mapper = |row: &rusqlite::Row| row.get(0);

            let rows = match key_type {
                Some(ktype) => stmt.query_map(params![ktype], row_mapper),
                None => stmt.query_map([], row_mapper),
            }
            .map_err(|e| DatabaseOperationError::SqliteError(e))?;

            let keys = rows
                .collect::<Result<Vec<String>, _>>()
                .map_err(|e| DatabaseOperationError::SqliteError(e))?;

            Ok(keys)
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_encryption_key_info(
        &self,
    ) -> Result<(String, String), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT encryption_keys.file_path, encryption_keys.sha256_hash
                 FROM configuration
                 JOIN encryption_keys ON configuration.encryption_key_id = \
                 encryption_keys.id
                 WHERE configuration.id = ? AND configuration.section = \
                 'profile'",
                params![self.profile.as_ref().map(|p| p.id).unwrap_or(0)],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    DatabaseOperationError::ApplicationError(
                        ApplicationError::InvalidUserConfiguration(
                            "No encryption key found for profile".to_string(),
                        ),
                    )
                }
                _ => DatabaseOperationError::SqliteError(e),
            })
        })
        .map_err(|e| match e {
            DatabaseOperationError::SqliteError(sqlite_err) => {
                ApplicationError::DatabaseError(sqlite_err.to_string())
            }
            DatabaseOperationError::ApplicationError(app_err) => app_err,
        })
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

    fn get_or_insert_encryption_key<'a>(
        &self,
        tx: &Transaction<'a>,
        key_path: &PathBuf,
        key_hash: &str,
    ) -> Result<i64, DatabaseOperationError> {
        // First, try to find an existing key with the same hash
        let existing_key_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM encryption_keys WHERE sha256_hash = ?",
                params![key_hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(DatabaseOperationError::SqliteError)?;

        match existing_key_id {
            Some(id) => Ok(id),
            None => {
                // If no existing key found, insert a new one
                tx.query_row(
                    "INSERT INTO encryption_keys (file_path, sha256_hash) \
                     VALUES (?, ?) RETURNING id",
                    params![key_path.to_str().unwrap(), key_hash],
                    |row| row.get(0),
                )
                .map_err(DatabaseOperationError::SqliteError)
            }
        }
    }
}
