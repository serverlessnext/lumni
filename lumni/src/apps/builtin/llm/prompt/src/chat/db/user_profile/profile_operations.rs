use std::path::PathBuf;
use std::sync::Arc;

use lumni::api::error::ApplicationError;
use lumni::Timestamp;
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::Value as JsonValue;
use tokio::time::{sleep, Duration};

use super::{
    DatabaseOperationError, EncryptionHandler, EncryptionMode, MaskMode,
    UserProfile, UserProfileDbHandler,
};
use crate::external as lumni;

impl UserProfileDbHandler {
    pub async fn create(
        &mut self,
        profile_name: &str,
        settings: &JsonValue,
    ) -> Result<UserProfile, ApplicationError> {
        // Simulate a 3-second delay
        //sleep(Duration::from_secs(3)).await;
        let timestamp = Timestamp::from_system_time().unwrap().as_millis();

        let encryption_key_id = self.get_or_create_encryption_key().await?;
        let processed_settings = self.process_settings(
            settings,
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
        let profile = db
            .process_queue_with_result(|tx| {
                tx.execute(
                    "INSERT INTO user_profiles (name, options, \
                     encryption_key_id, created_at) VALUES (?, ?, ?, ?)",
                    params![
                        profile_name,
                        json_string,
                        encryption_key_id,
                        timestamp
                    ],
                )
                .map_err(DatabaseOperationError::SqliteError)?;

                let id = tx.last_insert_rowid();
                Ok(UserProfile {
                    id,
                    name: profile_name.to_string(),
                })
            })
            .map_err(|e| match e {
                DatabaseOperationError::SqliteError(sqlite_err) => {
                    ApplicationError::DatabaseError(sqlite_err.to_string())
                }
                DatabaseOperationError::ApplicationError(app_err) => app_err,
            })?;

        Ok(profile)
    }

    pub async fn update(
        &mut self,
        profile: &UserProfile,
        new_settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        // Update profile settings
        self.update_profile_settings(profile, new_settings).await?;

        // Update the name if it has changed
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "UPDATE user_profiles SET name = ? WHERE id = ?",
                params![profile.name, profile.id],
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

        self.profile = Some(profile.clone());

        Ok(())
    }

    pub fn get_or_insert_encryption_key<'a>(
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
        profile: &UserProfile,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        log::debug!(
            "Getting settings for profile: {}:{} ({:?})",
            profile.id,
            profile.name,
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
                    params![profile.name],
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
                profile.clone(),
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
        profile: &UserProfile,
        new_name: &str,
    ) -> Result<(), ApplicationError> {
        log::debug!(
            "Renaming profile '{}' (ID: {}) to '{}'",
            profile.name,
            profile.id,
            new_name
        );
        if profile.name == new_name {
            return Ok(()); // No need to rename if the names are the same
        }

        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            // Perform the rename
            let updated_rows = tx.execute(
                "UPDATE user_profiles SET name = ? WHERE id = ?",
                params![new_name, profile.id],
            )?;

            if updated_rows == 0 {
                Err(DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "Profile '{}' (ID: {}) not found",
                        profile.name, profile.id
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
