use std::sync::Arc;

use base64::engine::general_purpose;
use base64::Engine as _;
use lumni::api::error::{ApplicationError, EncryptionError};
use rusqlite::{params, OptionalExtension};
use serde_json::{Map, Value as JsonValue};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex as TokioMutex;

use super::connector::{DatabaseConnector, DatabaseOperationError};
use super::encryption::EncryptionHandler;
use crate::external as lumni;

pub struct UserProfileDbHandler {
    profile_name: Option<String>,
    db: Arc<TokioMutex<DatabaseConnector>>,
    encryption_handler: Option<Arc<EncryptionHandler>>,
}

impl UserProfileDbHandler {
    pub fn new(
        profile_name: Option<String>,
        db: Arc<TokioMutex<DatabaseConnector>>,
        encryption_handler: Option<Arc<EncryptionHandler>>,
    ) -> Self {
        UserProfileDbHandler {
            profile_name,
            db,
            encryption_handler,
        }
    }

    pub fn get_profile_name(&self) -> Option<&String> {
        self.profile_name.as_ref()
    }

    pub fn set_profile_name(&mut self, profile_name: String) {
        self.profile_name = Some(profile_name);
    }

    pub async fn get_profile_settings(
        &self,
        profile_name: &str,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let result: Result<JsonValue, DatabaseOperationError> = {
                let (json_string, ssh_key_hash): (String, Option<String>) = tx
                    .query_row(
                        "SELECT options, ssh_key_hash FROM user_profiles \
                         WHERE name = ?",
                        params![profile_name],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|e| DatabaseOperationError::SqliteError(e))?;

                let settings: JsonValue = serde_json::from_str(&json_string)
                    .map_err(|e| {
                        DatabaseOperationError::ApplicationError(
                            ApplicationError::InvalidInput(format!(
                                "Invalid JSON: {}",
                                e
                            )),
                        )
                    })?;

                if let Some(hash) = ssh_key_hash {
                    self.verify_ssh_key_hash(&hash)
                        .map_err(DatabaseOperationError::ApplicationError)?;
                }

                self.process_settings(&settings, false, mask_encrypted)
                    .map_err(DatabaseOperationError::ApplicationError)
            };
            result
        })
        .map_err(|e| match e {
            DatabaseOperationError::SqliteError(sqlite_err) => {
                ApplicationError::DatabaseError(sqlite_err.to_string())
            }
            DatabaseOperationError::ApplicationError(app_err) => app_err,
        })
    }

    pub async fn create_or_update(
        &self,
        profile_name: &str,
        new_settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            // First, check if the profile exists and get its current data and is_default status
            let current_data: Option<(String, Option<String>, bool)> = tx
                .query_row(
                    "SELECT options, ssh_key_hash, is_default FROM \
                     user_profiles WHERE name = ?",
                    params![profile_name],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;

            let (merged_settings, ssh_key_hash, is_default) =
                if let Some((current_json, existing_hash, is_default)) =
                    current_data
                {
                    let mut current: JsonValue =
                        serde_json::from_str(&current_json).map_err(|e| {
                            ApplicationError::InvalidInput(format!(
                                "Invalid JSON: {}",
                                e
                            ))
                        })?;

                    // Merge settings, handling deletions
                    if let Some(current_obj) = current.as_object_mut() {
                        if let Some(new_obj) = new_settings.as_object() {
                            for (key, value) in new_obj {
                                if value.is_null() {
                                    current_obj.remove(key); // Remove the key if the new value is null
                                } else {
                                    current_obj
                                        .insert(key.clone(), value.clone()); // Otherwise, update or add the key-value pair
                                }
                            }
                        }
                    }

                    (current, existing_hash, is_default)
                } else {
                    let ssh_key_hash = self
                        .encryption_handler
                        .as_ref()
                        .and_then(|_| self.calculate_ssh_key_hash().ok());
                    (new_settings.clone(), ssh_key_hash, false) // New profiles are not default by default
                };

            let processed_settings =
                self.process_settings(&merged_settings, true, false)?;
            let json_string = serde_json::to_string(&processed_settings)
                .map_err(|e| {
                    ApplicationError::InvalidInput(format!(
                        "Failed to serialize JSON: {}",
                        e
                    ))
                })?;

            // Use INSERT OR REPLACE, but explicitly set the is_default status
            tx.execute(
                "INSERT OR REPLACE INTO user_profiles (name, options, \
                 ssh_key_hash, is_default) VALUES (?, ?, ?, ?)",
                params![profile_name, json_string, ssh_key_hash, is_default],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;

            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    fn process_settings(
        &self,
        value: &JsonValue,
        encrypt: bool,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        match value {
            JsonValue::Object(obj) => {
                let mut new_obj = Map::new();
                for (k, v) in obj {
                    if encrypt {
                        if let JsonValue::Object(inner_obj) = v {
                            if inner_obj.get("secure")
                                == Some(&JsonValue::Bool(true))
                            {
                                // Encrypt secure strings
                                if let Some(JsonValue::String(content)) =
                                    inner_obj.get("value")
                                {
                                    new_obj.insert(
                                        k.clone(),
                                        self.encrypt_value(
                                            &JsonValue::String(content.clone()),
                                        )?,
                                    );
                                }
                            } else {
                                // Don't encrypt regular strings
                                new_obj.insert(k.clone(), v.clone());
                            }
                        } else {
                            // Don't encrypt non-string values
                            new_obj.insert(k.clone(), v.clone());
                        }
                    } else {
                        // During get operation, handle decryption and masking
                        if Self::is_encrypted_value(v) {
                            if mask_encrypted {
                                new_obj.insert(
                                    k.clone(),
                                    JsonValue::String("*****".to_string()),
                                );
                            } else {
                                new_obj
                                    .insert(k.clone(), self.decrypt_value(v)?);
                            }
                        } else {
                            new_obj.insert(k.clone(), v.clone());
                        }
                    }
                }
                Ok(JsonValue::Object(new_obj))
            }
            JsonValue::Array(arr) => {
                let new_arr: Result<Vec<JsonValue>, _> = arr
                    .iter()
                    .map(|v| self.process_settings(v, encrypt, mask_encrypted))
                    .collect();
                Ok(JsonValue::Array(new_arr?))
            }
            _ => Ok(value.clone()),
        }
    }

    fn encrypt_value(
        &self,
        value: &JsonValue,
    ) -> Result<JsonValue, ApplicationError> {
        if let Some(ref encryption_handler) = self.encryption_handler {
            if let JsonValue::String(content) = value {
                let (encrypted_content, encryption_key) = encryption_handler
                    .encrypt_string(content)
                    .map_err(|e| {
                        ApplicationError::EncryptionError(
                            EncryptionError::Other(Box::new(e)),
                        )
                    })?;

                Ok(JsonValue::Object(Map::from_iter(vec![
                    (
                        "content".to_string(),
                        JsonValue::String(encrypted_content),
                    ),
                    (
                        "encryption_key".to_string(),
                        JsonValue::String(encryption_key),
                    ),
                ])))
            } else {
                Ok(value.clone())
            }
        } else {
            Ok(value.clone())
        }
    }

    fn decrypt_value(
        &self,
        value: &JsonValue,
    ) -> Result<JsonValue, ApplicationError> {
        if let Some(ref encryption_handler) = self.encryption_handler {
            if let JsonValue::Object(obj) = value {
                if let (
                    Some(JsonValue::String(content)),
                    Some(JsonValue::String(encrypted_key)),
                ) = (obj.get("content"), obj.get("encryption_key"))
                {
                    let decrypted = encryption_handler
                        .decrypt_string(content, encrypted_key)?;
                    Ok(JsonValue::String(decrypted))
                } else {
                    Ok(value.clone())
                }
            } else {
                Ok(value.clone())
            }
        } else {
            Ok(value.clone())
        }
    }

    fn is_encrypted_value(value: &JsonValue) -> bool {
        if let JsonValue::Object(obj) = value {
            obj.contains_key("content") && obj.contains_key("encryption_key")
        } else {
            false
        }
    }
    pub async fn delete_profile(
        &self,
        profile_name: &str,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            tx.execute(
                "DELETE FROM user_profiles WHERE name = ?",
                params![profile_name],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    pub async fn list_profiles(&self) -> Result<Vec<String>, ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            let mut stmt = tx
                .prepare("SELECT name FROM user_profiles")
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            let profiles = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?
                .collect::<Result<Vec<String>, _>>()
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(profiles)
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_default_profile(
        &self,
    ) -> Result<Option<String>, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT name FROM user_profiles WHERE is_default = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DatabaseOperationError::SqliteError(e))
        })
        .map_err(ApplicationError::from)
    }

    pub async fn set_default_profile(
        &self,
        profile_name: &str,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            tx.execute(
                "UPDATE user_profiles SET is_default = 0 WHERE is_default = 1",
                [],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            tx.execute(
                "UPDATE user_profiles SET is_default = 1 WHERE name = ?",
                params![profile_name],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    fn calculate_ssh_key_hash(&self) -> Result<String, ApplicationError> {
        if let Some(ref encryption_handler) = self.encryption_handler {
            let ssh_private_key = encryption_handler
                .get_ssh_private_key()
                .map_err(|e| ApplicationError::EncryptionError(e.into()))?;
            let mut hasher = Sha256::new();
            hasher.update(ssh_private_key);
            let result = hasher.finalize();
            Ok(general_purpose::STANDARD.encode(result))
        } else {
            Err(ApplicationError::NotReady(
                "No encryption handler available".to_string(),
            ))
        }
    }

    fn verify_ssh_key_hash(
        &self,
        stored_hash: &str,
    ) -> Result<(), ApplicationError> {
        let current_hash = self.calculate_ssh_key_hash()?;
        if current_hash != stored_hash {
            return Err(ApplicationError::InvalidInput(
                "SSH key hash mismatch".to_string(),
            ));
        }
        Ok(())
    }
}
