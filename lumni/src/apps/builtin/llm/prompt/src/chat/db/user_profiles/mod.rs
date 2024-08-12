use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use dirs::home_dir;
use libc::EPERM;
use lumni::api::error::{ApplicationError, EncryptionError};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Map, Value as JsonValue};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex as TokioMutex;

use super::connector::{DatabaseConnector, DatabaseOperationError};
use super::encryption::EncryptionHandler;
use crate::external as lumni;

#[derive(Debug)]
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

    pub fn get_encryption_handler(&self) -> Option<&Arc<EncryptionHandler>> {
        self.encryption_handler.as_ref()
    }

    pub fn set_encryption_handler(
        &mut self,
        encryption_handler: Arc<EncryptionHandler>,
    ) {
        self.encryption_handler = Some(encryption_handler);
    }

    pub async fn profile_exists(
        &self,
        profile_name: &str,
    ) -> Result<bool, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let count: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM user_profiles WHERE name = ?",
                    params![profile_name],
                    |row| row.get(0),
                )
                .map_err(DatabaseOperationError::SqliteError)?;
            Ok(count > 0)
        })
        .map_err(ApplicationError::from)
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
                    // If the encryption_key is empty, return the content as is
                    if encrypted_key.is_empty() {
                        return Ok(JsonValue::String(content.clone()));
                    }

                    match encryption_handler
                        .decrypt_string(content, encrypted_key)
                    {
                        Ok(decrypted) => Ok(JsonValue::String(decrypted)),
                        Err(e) => {
                            eprintln!("Decryption error: {:?}", e);
                            eprintln!(
                                "Content length: {}, Key length: {}",
                                content.len(),
                                encrypted_key.len()
                            );
                            Err(e)
                        }
                    }
                } else {
                    eprintln!("Invalid encrypted value format");
                    Ok(value.clone())
                }
            } else {
                eprintln!("Value is not an object: {:?}", value);
                Ok(value.clone())
            }
        } else {
            eprintln!("No encryption handler available");
            Ok(value.clone())
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
                    new_obj.insert(
                        k.clone(),
                        self.process_value(v, encrypt, mask_encrypted)?,
                    );
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
                if let Some(ref encryption_handler) = self.encryption_handler {
                    self.encrypt_value(content)
                } else {
                    Err(ApplicationError::EncryptionError(
                        EncryptionError::Other(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "No encryption handler available for encryption",
                        ))),
                    ))
                }
            } else {
                Err(ApplicationError::InvalidInput(
                    "Invalid secure string format".to_string(),
                ))
            }
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
            if let Some(obj) = value.as_object() {
                if let (
                    Some(JsonValue::String(content)),
                    Some(JsonValue::String(encrypted_key)),
                ) = (obj.get("content"), obj.get("encryption_key"))
                {
                    // If the encryption_key is empty, return the content as is
                    if encrypted_key.is_empty() {
                        return Ok(JsonValue::String(content.clone()));
                    }

                    // Otherwise, proceed with normal decryption logic
                    if self.encryption_handler.is_some() {
                        if mask_encrypted {
                            Ok(JsonValue::String("*****".to_string()))
                        } else {
                            self.decrypt_value(value)
                        }
                    } else {
                        Ok(JsonValue::String("*****".to_string())) // Always mask if no encryption handler
                    }
                } else {
                    // Invalid format for encrypted value
                    Err(ApplicationError::InvalidInput(
                        "Invalid encrypted value format".to_string(),
                    ))
                }
            } else {
                // Value marked as encrypted but not an object
                Err(ApplicationError::InvalidInput(
                    "Encrypted value must be an object".to_string(),
                ))
            }
        } else {
            Ok(value.clone())
        }
    }

    fn generate_encryption_key(
        &self,
        profile_name: &str,
    ) -> Result<(EncryptionHandler, PathBuf, String), ApplicationError> {
        let key_dir =
            home_dir().unwrap_or_default().join(".lumni").join("keys");
        fs::create_dir_all(&key_dir)
            .map_err(|e| ApplicationError::IOError(e))?;
        let key_path = key_dir.join(format!("{}_key.pem", profile_name));

        let encryption_handler =
            EncryptionHandler::generate_private_key(&key_path, 2048, None)?;
        let key_hash = EncryptionHandler::get_private_key_hash(&key_path)?;

        Ok((encryption_handler, key_path, key_hash))
    }

    pub async fn create_or_update(
        &mut self,
        profile_name: &str,
        new_settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        let (encryption_key_id, merged_settings, new_encryption_handler) = {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                let existing_profile: Option<(i64, String, i64)> = tx
                    .query_row(
                        "SELECT id, options, encryption_key_id FROM \
                         user_profiles WHERE name = ?",
                        params![profile_name],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()
                    .map_err(DatabaseOperationError::SqliteError)?;

                match existing_profile {
                    Some((_, existing_options, existing_key_id)) => {
                        let merged = self.merge_settings(
                            Some(existing_options),
                            new_settings,
                        )?;
                        Ok((existing_key_id, merged, None))
                    }
                    None => {
                        let (new_encryption_handler, key_path, key_hash) =
                            self.generate_encryption_key(profile_name)?;

                        let key_id: i64 = tx
                            .query_row(
                                "INSERT INTO encryption_keys (file_path, \
                                 sha256_hash) VALUES (?, ?) RETURNING id",
                                params![key_path.to_str().unwrap(), key_hash],
                                |row| row.get(0),
                            )
                            .map_err(DatabaseOperationError::SqliteError)?;

                        eprintln!("key_id for new profile: {}", key_id);
                        Ok((
                            key_id,
                            new_settings.clone(),
                            Some(new_encryption_handler),
                        ))
                    }
                }
            })
            .map_err(|e| match e {
                DatabaseOperationError::SqliteError(sqlite_err) => {
                    ApplicationError::DatabaseError(sqlite_err.to_string())
                }
                DatabaseOperationError::ApplicationError(app_err) => app_err,
            })?
        };
        if let Some(handler) = new_encryption_handler {
            self.encryption_handler = Some(Arc::new(handler));
        } else if self.encryption_handler.is_none() {
            // If we don't have a new handler and the existing one is None, try to load it
            let encryption_handler =
                self.load_encryption_handler(encryption_key_id).await?;
            self.encryption_handler = Some(Arc::new(encryption_handler));
        }

        // Verify that we now have an encryption handler
        if self.encryption_handler.is_none() {
            return Err(ApplicationError::EncryptionError(
                EncryptionError::Other(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to set up encryption handler",
                ))),
            ));
        }

        let processed_settings =
            self.process_settings(&merged_settings, true, false)?;

        let json_string =
            serde_json::to_string(&processed_settings).map_err(|e| {
                ApplicationError::InvalidInput(format!(
                    "Failed to serialize JSON: {}",
                    e
                ))
            })?;

        eprintln!("Encryption_key_id: {}", encryption_key_id);
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "INSERT OR REPLACE INTO user_profiles (name, options, \
                 encryption_key_id) VALUES (?, ?, ?)",
                params![profile_name, json_string, encryption_key_id],
            )
            .map_err(DatabaseOperationError::SqliteError)?;
            Ok(())
        })
        .map_err(|e| match e {
            DatabaseOperationError::SqliteError(sqlite_err) => {
                ApplicationError::DatabaseError(sqlite_err.to_string())
            }
            DatabaseOperationError::ApplicationError(app_err) => app_err,
        })?;

        // Update self after database operations
        self.profile_name = Some(profile_name.to_string());

        Ok(())
    }

    async fn load_encryption_handler(
        &self,
        encryption_key_id: i64,
    ) -> Result<EncryptionHandler, ApplicationError> {
        let mut db = self.db.lock().await;
        let key_path: String = db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT file_path FROM encryption_keys WHERE id = ?",
                params![encryption_key_id],
                |row| row.get(0),
            )
            .map_err(DatabaseOperationError::SqliteError)
        })?;

        EncryptionHandler::new_from_path(&PathBuf::from(key_path))?.ok_or_else(
            || {
                ApplicationError::EncryptionError(EncryptionError::Other(
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Failed to create encryption handler from key file",
                    )),
                ))
            },
        )
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

    pub async fn get_profile_settings(
        &mut self,
        profile_name: &str,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        let (json_string, key_hash, key_path): (String, String, String) = {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                tx.query_row(
                    "SELECT user_profiles.options, \
                     encryption_keys.sha256_hash, encryption_keys.file_path
                     FROM user_profiles
                     JOIN encryption_keys ON user_profiles.encryption_key_id = \
                     encryption_keys.id
                     WHERE user_profiles.name = ?",
                    params![profile_name],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(DatabaseOperationError::SqliteError)
            })?
        };

        if self.encryption_handler.is_none() {
            let encryption_handler =
                EncryptionHandler::new_from_path(&PathBuf::from(&key_path))?
                    .ok_or_else(|| {
                        ApplicationError::InvalidInput(
                            "Failed to load encryption handler".to_string(),
                        )
                    })?;
            self.encryption_handler = Some(Arc::new(encryption_handler));
        }
        self.verify_encryption_key_hash(&key_hash)?;
        let settings: JsonValue =
            serde_json::from_str(&json_string).map_err(|e| {
                ApplicationError::InvalidInput(format!("Invalid JSON: {}", e))
            })?;
        self.process_settings_with_metadata(&settings, false, mask_encrypted)
    }

    fn process_settings_with_metadata(
        &self,
        value: &JsonValue,
        encrypt: bool,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        match value {
            JsonValue::Object(obj) => {
                let mut new_obj = Map::new();
                for (k, v) in obj {
                    let processed = self.process_value_with_metadata(
                        v,
                        encrypt,
                        mask_encrypted,
                    )?;
                    new_obj.insert(k.clone(), processed);
                }
                Ok(JsonValue::Object(new_obj))
            }
            JsonValue::Array(arr) => {
                let new_arr: Result<Vec<JsonValue>, _> = arr
                    .iter()
                    .map(|v| {
                        self.process_settings_with_metadata(
                            v,
                            encrypt,
                            mask_encrypted,
                        )
                    })
                    .collect();
                Ok(JsonValue::Array(new_arr?))
            }
            _ => Ok(value.clone()),
        }
    }

    fn process_value_with_metadata(
        &self,
        value: &JsonValue,
        encrypt: bool,
        mask_encrypted: bool,
    ) -> Result<JsonValue, ApplicationError> {
        if encrypt {
            self.handle_encryption(value)
        } else {
            let decrypted = self.handle_decryption(value, mask_encrypted)?;
            if Self::is_encrypted_value(value) {
                Ok(json!({
                    "value": decrypted,
                    "was_encrypted": true
                }))
            } else {
                Ok(decrypted)
            }
        }
    }

    fn verify_encryption_key_hash(
        &self,
        stored_hash: &str,
    ) -> Result<(), ApplicationError> {
        let current_hash = self.calculate_current_key_hash()?;
        if current_hash != stored_hash {
            return Err(ApplicationError::InvalidInput(
                "Encryption key hash mismatch".to_string(),
            ));
        }

        Ok(())
    }

    fn calculate_current_key_hash(&self) -> Result<String, ApplicationError> {
        if let Some(ref encryption_handler) = self.encryption_handler {
            let key_data = encryption_handler.get_private_key_pem()?;
            let mut hasher = Sha256::new();
            hasher.update(key_data.as_bytes());
            Ok(format!("{:x}", hasher.finalize()))
        } else {
            Err(ApplicationError::InvalidInput(
                "No encryption handler available".to_string(),
            ))
        }
    }
}

impl UserProfileDbHandler {
    pub async fn register_encryption_key(
        &self,
        name: &str,
        file_path: &PathBuf,
        key_type: &str,
    ) -> Result<(), ApplicationError> {
        let hash = EncryptionHandler::get_private_key_hash(file_path)?;
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "INSERT INTO encryption_keys (name, file_path, sha256_hash, \
                 key_type) VALUES (?, ?, ?, ?)",
                params![name, file_path.to_str().unwrap(), hash, key_type],
            )
            .map_err(|e| DatabaseOperationError::SqliteError(e))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_encryption_key(
        &self,
        name: &str,
    ) -> Result<(String, String, String), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT file_path, sha256_hash, key_type FROM encryption_keys \
                 WHERE name = ?",
                params![name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
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

    pub async fn export_profile_settings(
        &mut self,
        profile_name: &str,
    ) -> Result<JsonValue, ApplicationError> {
        let settings = self.get_profile_settings(profile_name, false).await?;
        Ok(self.create_export_json(&settings))
    }

    fn create_export_json(&self, settings: &JsonValue) -> JsonValue {
        match settings {
            JsonValue::Object(obj) => {
                let mut parameters = Vec::new();
                for (key, value) in obj {
                    let (param_type, param_value) = if let Some(metadata) =
                        value.as_object()
                    {
                        if metadata.get("was_encrypted")
                            == Some(&JsonValue::Bool(true))
                        {
                            (
                                "SecureString",
                                metadata.get("value").unwrap_or(value).clone(),
                            )
                        } else {
                            ("String", value.clone())
                        }
                    } else {
                        ("String", value.clone())
                    };
                    parameters.push(json!({
                        "Key": key,
                        "Value": param_value,
                        "Type": param_type
                    }));
                }
                json!({
                    "Parameters": parameters
                })
            }
            _ => JsonValue::Null,
        }
    }

    fn is_encrypted_value(value: &JsonValue) -> bool {
        if let Some(obj) = value.as_object() {
            obj.contains_key("content") && obj.contains_key("encryption_key")
        } else {
            false
        }
    }

    fn get_decrypted_value(
        &self,
        value: &JsonValue,
    ) -> Result<JsonValue, ApplicationError> {
        if let Some(obj) = value.as_object() {
            if let (
                Some(JsonValue::String(content)),
                Some(JsonValue::String(encrypted_key)),
            ) = (obj.get("content"), obj.get("encryption_key"))
            {
                if encrypted_key.is_empty() {
                    // If encryption_key is empty, return content as is
                    Ok(JsonValue::String(content.clone()))
                } else if let Some(ref encryption_handler) =
                    self.encryption_handler
                {
                    // Decrypt the value
                    let decrypted = encryption_handler
                        .decrypt_string(content, encrypted_key)?;
                    Ok(JsonValue::String(decrypted))
                } else {
                    // No encryption handler available
                    Err(ApplicationError::EncryptionError(
                        EncryptionError::Other(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "No encryption handler available for decryption",
                        ))),
                    ))
                }
            } else {
                Err(ApplicationError::InvalidInput(
                    "Invalid encrypted value format".to_string(),
                ))
            }
        } else {
            Ok(value.clone())
        }
    }
}
