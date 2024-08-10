use std::sync::Arc;

use base64::engine::general_purpose;
use base64::Engine as _;
use lumni::api::error::{ApplicationError, EncryptionError};
use rusqlite::{params, OptionalExtension, Transaction};
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

    fn encrypt_value(
        &self,
        content: &str,
    ) -> Result<JsonValue, ApplicationError> {
        if let Some(ref encryption_handler) = self.encryption_handler {
            let (encrypted_content, encryption_key) =
                encryption_handler.encrypt_string(content).map_err(|e| {
                    ApplicationError::EncryptionError(EncryptionError::Other(
                        Box::new(e),
                    ))
                })?;

            Ok(JsonValue::Object(Map::from_iter(vec![
                ("content".to_string(), JsonValue::String(encrypted_content)),
                (
                    "encryption_key".to_string(),
                    JsonValue::String(encryption_key),
                ),
            ])))
        } else {
            Ok(JsonValue::String(content.to_string()))
        }
    }

    fn decrypt_value(
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
        if let Some(obj) = value.as_object() {
            obj.contains_key("content") && obj.contains_key("encryption_key")
        } else {
            false
        }
    }

    fn is_marked_for_encryption(value: &JsonValue) -> bool {
        if let Some(obj) = value.as_object() {
            obj.contains_key("content")
                && obj.get("encryption_key")
                    == Some(&JsonValue::String("".to_string()))
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

    pub async fn get_profile_settings(
        &self,
        profile_name: &str,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            self.fetch_and_process_settings(tx, profile_name, mask_encrypted)
        })
        .map_err(|e| e.into())
    }

    fn fetch_and_process_settings(
        &self,
        tx: &Transaction,
        profile_name: &str,
        mask_encrypted: bool,
    ) -> Result<JsonValue, DatabaseOperationError> {
        let (json_string, ssh_key_hash) =
            self.fetch_profile_data(tx, profile_name)?;
        let settings: JsonValue = self.parse_json(&json_string)?;

        if let Some(hash) = ssh_key_hash {
            self.verify_ssh_key_hash(&hash)
                .map_err(DatabaseOperationError::ApplicationError)?;
        }

        self.process_settings(&settings, false, mask_encrypted)
            .map_err(DatabaseOperationError::ApplicationError)
    }

    fn fetch_profile_data(
        &self,
        tx: &Transaction,
        profile_name: &str,
    ) -> Result<(String, Option<String>), DatabaseOperationError> {
        tx.query_row(
            "SELECT options, ssh_key_hash FROM user_profiles WHERE name = ?",
            params![profile_name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(DatabaseOperationError::SqliteError)
    }

    fn parse_json(
        &self,
        json_string: &str,
    ) -> Result<JsonValue, DatabaseOperationError> {
        serde_json::from_str(json_string).map_err(|e| {
            DatabaseOperationError::ApplicationError(
                ApplicationError::InvalidInput(format!("Invalid JSON: {}", e)),
            )
        })
    }

    fn process_settings(
        &self,
        value: &JsonValue,
        encrypt: bool,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        match value {
            JsonValue::Object(obj) => {
                self.process_object(obj, encrypt, mask_encrypted)
            }
            JsonValue::Array(arr) => {
                self.process_array(arr, encrypt, mask_encrypted)
            }
            _ => Ok(value.clone()),
        }
    }

    fn process_object(
        &self,
        obj: &Map<String, JsonValue>,
        encrypt: bool,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        let mut new_obj = Map::new();
        for (k, v) in obj {
            new_obj.insert(
                k.clone(),
                self.process_value(v, encrypt, mask_encrypted)?,
            );
        }
        Ok(JsonValue::Object(new_obj))
    }

    fn process_array(
        &self,
        arr: &[JsonValue],
        encrypt: bool,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        let new_arr: Result<Vec<JsonValue>, _> = arr
            .iter()
            .map(|v| self.process_settings(v, encrypt, mask_encrypted))
            .collect();
        Ok(JsonValue::Array(new_arr?))
    }

    fn process_value(
        &self,
        value: &JsonValue,
        encrypt: bool,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        if encrypt {
            self.handle_encryption(value)
        } else {
            self.handle_decryption(value, mask_encrypted)
        }
    }

    fn handle_encryption(
        &self,
        value: &JsonValue,
    ) -> Result<JsonValue, ApplicationError> {
        if Self::is_marked_for_encryption(value) {
            if let Some(JsonValue::String(content)) = value.get("content") {
                self.encrypt_value(content)
            } else {
                Err(ApplicationError::InvalidInput(
                    "Invalid secure string format".to_string(),
                ))
            }
        } else if !Self::is_encrypted_value(value) {
            Ok(value.clone())
        } else {
            Ok(value.clone())
        }
    }

    fn handle_decryption(
        &self,
        value: &JsonValue,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        if Self::is_encrypted_value(value) {
            if mask_encrypted {
                Ok(JsonValue::String("*****".to_string()))
            } else {
                self.decrypt_value(value)
            }
        } else {
            Ok(value.clone())
        }
    }
    pub async fn create_or_update(
        &self,
        profile_name: &str,
        new_settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            self.perform_create_or_update(tx, profile_name, new_settings)
        })
        .map_err(|e| match e {
            DatabaseOperationError::SqliteError(sqlite_err) => {
                ApplicationError::DatabaseError(sqlite_err.to_string())
            }
            DatabaseOperationError::ApplicationError(app_err) => app_err,
        })
    }

    fn perform_create_or_update(
        &self,
        tx: &Transaction,
        profile_name: &str,
        new_settings: &JsonValue,
    ) -> Result<(), DatabaseOperationError> {
        let current_data = self.fetch_current_profile_data(tx, profile_name)?;
        let merged_settings =
            self.merge_settings(current_data, new_settings)?;
        let processed_settings =
            self.process_settings(&merged_settings, true, false)?;
        self.save_profile_settings(tx, profile_name, &processed_settings)?;
        Ok(())
    }

    fn fetch_current_profile_data(
        &self,
        tx: &Transaction,
        profile_name: &str,
    ) -> Result<Option<String>, DatabaseOperationError> {
        tx.query_row(
            "SELECT options FROM user_profiles WHERE name = ?",
            params![profile_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| DatabaseOperationError::SqliteError(e))
    }

    fn merge_settings(
        &self,
        current_data: Option<String>,
        new_settings: &JsonValue,
    ) -> Result<JsonValue, DatabaseOperationError> {
        if let Some(current_json) = current_data {
            let current: JsonValue = serde_json::from_str(&current_json)
                .map_err(|e| {
                    DatabaseOperationError::ApplicationError(
                        ApplicationError::InvalidInput(format!(
                            "Invalid JSON: {}",
                            e
                        )),
                    )
                })?;

            let mut merged = current.clone();
            if let (Some(merged_obj), Some(new_obj)) =
                (merged.as_object_mut(), new_settings.as_object())
            {
                for (key, new_value) in new_obj {
                    self.merge_setting(merged_obj, key, new_value, &current)?;
                }
            }
            Ok(merged)
        } else {
            Ok(new_settings.clone())
        }
    }

    fn merge_setting(
        &self,
        merged_obj: &mut Map<String, JsonValue>,
        key: &String,
        new_value: &JsonValue,
        current: &JsonValue,
    ) -> Result<(), DatabaseOperationError> {
        if new_value.is_null() {
            merged_obj.remove(key);
        } else {
            let current_value = current.get(key);
            let is_currently_encrypted =
                current_value.map(Self::is_encrypted_value).unwrap_or(false);
            let is_new_value_marked_for_encryption =
                Self::is_marked_for_encryption(new_value);

            if is_currently_encrypted {
                self.handle_encrypted_value(
                    merged_obj,
                    key,
                    new_value,
                    is_new_value_marked_for_encryption,
                )?;
            } else if is_new_value_marked_for_encryption {
                merged_obj.insert(key.clone(), new_value.clone());
            } else {
                merged_obj.insert(key.clone(), new_value.clone());
            }
        }
        Ok(())
    }

    fn handle_encrypted_value(
        &self,
        merged_obj: &mut Map<String, JsonValue>,
        key: &String,
        new_value: &JsonValue,
        is_new_value_marked_for_encryption: bool,
    ) -> Result<(), DatabaseOperationError> {
        if is_new_value_marked_for_encryption {
            merged_obj.insert(key.clone(), new_value.clone());
        } else if let Some(content) = new_value.as_str() {
            let encrypted = self
                .encrypt_value(content)
                .map_err(DatabaseOperationError::ApplicationError)?;
            merged_obj.insert(key.clone(), encrypted);
        }
        Ok(())
    }

    fn save_profile_settings(
        &self,
        tx: &Transaction,
        profile_name: &str,
        settings: &JsonValue,
    ) -> Result<(), DatabaseOperationError> {
        let json_string = serde_json::to_string(settings).map_err(|e| {
            DatabaseOperationError::ApplicationError(
                ApplicationError::InvalidInput(format!(
                    "Failed to serialize JSON: {}",
                    e
                )),
            )
        })?;

        tx.execute(
            "INSERT OR REPLACE INTO user_profiles (name, options) VALUES (?, \
             ?)",
            params![profile_name, json_string],
        )
        .map_err(DatabaseOperationError::SqliteError)?;

        Ok(())
    }
}
