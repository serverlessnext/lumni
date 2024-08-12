use std::path::PathBuf;
use std::sync::Arc;

use lumni::api::error::{ApplicationError, EncryptionError};
use rusqlite::{params, OptionalExtension};
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
                            Self::generate_encryption_key(profile_name)?;

                        let key_id: i64 = tx
                            .query_row(
                                "INSERT INTO encryption_keys (file_path, \
                                 sha256_hash) VALUES (?, ?) RETURNING id",
                                params![key_path.to_str().unwrap(), key_hash],
                                |row| row.get(0),
                            )
                            .map_err(DatabaseOperationError::SqliteError)?;

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

        if let Some(encryption_handler) = new_encryption_handler {
            self.encryption_handler = Some(Arc::new(encryption_handler));
        } else if self.encryption_handler.is_none() {
            let encryption_handler =
                self.load_encryption_handler(encryption_key_id).await?;
            self.encryption_handler = Some(Arc::new(encryption_handler));
        }

        if self.encryption_handler.is_none() {
            return Err(ApplicationError::EncryptionError(
                EncryptionError::Other(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to set up encryption handler",
                ))),
            ));
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

    pub async fn get_profile_settings(
        &mut self,
        profile_name: &str,
        mask_mode: MaskMode,
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
        self.process_settings_with_metadata(
            &settings,
            EncryptionMode::Decrypt,
            mask_mode,
        )
    }
}
