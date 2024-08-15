use std::path::PathBuf;
use std::sync::Arc;

use lumni::api::error::ApplicationError;
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::Value as JsonValue;

use super::{
    DatabaseOperationError, EncryptionHandler, EncryptionMode, MaskMode,
    UserProfileDbHandler,
};
use crate::external as lumni;

impl UserProfileDbHandler {
    pub async fn create_or_update(
        &mut self,
        profile_name: &str,
        new_settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        let has_encryption_handler = self.encryption_handler.is_some();
        let mut created_encryption_handler: Option<EncryptionHandler> = None;

        let (encryption_key_id, merged_settings) = {
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

                        // if encryption handler is not available, create a new one from the key path
                        if !has_encryption_handler {
                            let key_path: String = tx
                                .query_row(
                                    "SELECT file_path FROM encryption_keys \
                                     WHERE id = ?",
                                    params![existing_key_id],
                                    |row| row.get(0),
                                )
                                .map_err(DatabaseOperationError::SqliteError)?;

                            let encryption_handler =
                                EncryptionHandler::new_from_path(
                                    &PathBuf::from(&key_path),
                                )?
                                .ok_or_else(
                                    || {
                                        ApplicationError::InvalidInput(
                                            "Failed to load encryption handler"
                                                .to_string(),
                                        )
                                    },
                                )?;
                            created_encryption_handler =
                                Some(encryption_handler);
                        }
                        Ok((existing_key_id, merged))
                    }
                    None => {
                        if !has_encryption_handler {
                            let (new_encryption_handler, key_path, key_hash) =
                                Self::generate_encryption_key(profile_name)?;
                            let key_id = self.get_or_insert_encryption_key(
                                tx, &key_path, &key_hash,
                            )?;
                            created_encryption_handler =
                                Some(new_encryption_handler);
                            Ok((key_id, new_settings.clone()))
                        } else {
                            let key_path = self
                                .encryption_handler
                                .as_ref()
                                .unwrap()
                                .get_key_path();
                            let key_hash =
                                EncryptionHandler::get_private_key_hash(
                                    &key_path,
                                )?;
                            let key_id = self.get_or_insert_encryption_key(
                                tx, &key_path, &key_hash,
                            )?;
                            Ok((key_id, new_settings.clone()))
                        }
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

        if self.encryption_handler.is_none() {
            // Set new encryption handler outside the closure
            if let Some(new_encryption_handler) = created_encryption_handler {
                // use method as it protects against overwriting existing encryption configuration
                self.set_profile_with_encryption_handler(
                    profile_name.to_string(),
                    Arc::new(new_encryption_handler),
                )?;
            } else {
                return Err(ApplicationError::InvalidInput(
                    "Failed to create encryption handler".to_string(),
                ));
            }
        }

        let processed_settings = self.process_settings(
            &merged_settings,
            EncryptionMode::Encrypt,
            MaskMode::Unmask,
        )?;

        let json_string =
            serde_json::to_string(&processed_settings).map_err(|e| {
                ApplicationError::InvalidInput(format!(
                    "Failed to serialize JSON: {}",
                    e
                ))
            })?;

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

        self.profile_name = Some(profile_name.to_string());

        Ok(())
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

    pub async fn get_profile_settings(
        &mut self,
        profile_name: &str,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        log::debug!(
            "Getting settings for profile: {} ({:?})",
            profile_name,
            mask_mode
        );
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
        if mask_mode == MaskMode::Unmask && self.encryption_handler.is_none() {
            // encryption handled required for decryption
            let encryption_handler =
                EncryptionHandler::new_from_path(&PathBuf::from(&key_path))?
                    .ok_or_else(|| {
                        ApplicationError::InvalidInput(
                            "Failed to load encryption handler".to_string(),
                        )
                    })?;
            self.set_profile_with_encryption_handler(
                profile_name.to_string(),
                Arc::new(encryption_handler),
            )?;
            self.verify_encryption_key_hash(&key_hash)?;
        }
        let settings: JsonValue =
            serde_json::from_str(&json_string).map_err(|e| {
                ApplicationError::InvalidInput(format!("Invalid JSON: {}", e))
            })?;
        self.process_settings_with_metadata(
            &settings,
            EncryptionMode::Decrypt,
            mask_mode,
        )
    }

    pub async fn rename_profile(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), ApplicationError> {
        log::debug!("Renaming profile '{}' to '{}'", old_name, new_name);
        if old_name == new_name {
            return Ok(()); // No need to rename if the names are the same
        }

        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            // Check if the new name already exists
            let exists: bool = tx
                .query_row(
                    "SELECT 1 FROM user_profiles WHERE name = ?",
                    params![new_name],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if exists {
                return Err(DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "Profile '{}' already exists",
                        new_name
                    )),
                ));
            }

            // Perform the rename
            let updated_rows = tx.execute(
                "UPDATE user_profiles SET name = ? WHERE name = ?",
                params![new_name, old_name],
            )?;

            if updated_rows == 0 {
                Err(DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "Profile '{}' not found",
                        old_name
                    )),
                ))
            } else {
                Ok(())
            }
        })
        .map_err(|e| match e {
            DatabaseOperationError::SqliteError(sqlite_err) => {
                ApplicationError::DatabaseError(sqlite_err.to_string())
            }
            DatabaseOperationError::ApplicationError(app_err) => app_err,
        })
    }
}
