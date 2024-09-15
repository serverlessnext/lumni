use lumni::api::error::ApplicationError;
use lumni::Timestamp;
use rusqlite::{params, OptionalExtension};
use serde_json::Value as JsonValue;

use super::{
    DatabaseOperationError, EncryptionMode, MaskMode, UserProfile,
    UserProfileDbHandler,
};
use crate::external as lumni;

impl UserProfileDbHandler {
    pub async fn create_profile(
        &mut self,
        profile_name: &str,
        settings: &JsonValue,
    ) -> Result<UserProfile, ApplicationError> {
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

    pub async fn update_profile(
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
        let json_string: String = {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                tx.query_row(
                    "SELECT user_profiles.options FROM user_profiles
                     WHERE user_profiles.name = ?",
                    params![profile.name],
                    |row| Ok((row.get(0)?)),
                )
                .map_err(DatabaseOperationError::SqliteError)
            })?
        };
        if mask_mode == MaskMode::Unmask && self.encryption_handler.is_none() {
            return Err(ApplicationError::InvalidInput(
                "Encryption handler not set".to_string(),
            ));
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

    async fn update_profile_settings(
        &mut self,
        profile: &UserProfile,
        new_settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        // Retrieve existing settings and merge with new settings
        let existing_settings =
            self.get_profile_settings(profile, MaskMode::Unmask).await?;
        let merged_settings =
            self.merge_settings(&existing_settings, new_settings)?;

        let processed_settings = self.process_settings(
            &merged_settings,
            EncryptionMode::Encrypt,
            MaskMode::Unmask,
        )?;

        // Serialize the processed settings
        let json_string =
            serde_json::to_string(&processed_settings).map_err(|e| {
                ApplicationError::InvalidInput(format!(
                    "Failed to serialize JSON: {}",
                    e
                ))
            })?;

        // Update the database
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let updated_rows = tx
                .execute(
                    "UPDATE user_profiles SET options = ? WHERE id = ?",
                    params![json_string, profile.id],
                )
                .map_err(DatabaseOperationError::SqliteError)?;

            if updated_rows == 0 {
                return Err(DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "Profile with id {} not found",
                        profile.id
                    )),
                ));
            }

            Ok(())
        })
        .map_err(|e| match e {
            DatabaseOperationError::SqliteError(sqlite_err) => {
                ApplicationError::DatabaseError(sqlite_err.to_string())
            }
            DatabaseOperationError::ApplicationError(app_err) => app_err,
        })?;

        Ok(())
    }

    pub async fn get_profile_by_id(
        &self,
        id: i64,
    ) -> Result<Option<UserProfile>, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT id, name FROM user_profiles WHERE id = ?",
                params![id],
                |row| {
                    Ok(UserProfile {
                        id: row.get(0)?,
                        name: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|e| DatabaseOperationError::SqliteError(e))
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_profiles_by_name(
        &self,
        name: &str,
    ) -> Result<Vec<UserProfile>, ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            let mut stmt = tx
                .prepare("SELECT id, name FROM user_profiles WHERE name = ?")
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            let profiles = stmt
                .query_map(params![name], |row| {
                    Ok(UserProfile {
                        id: row.get(0)?,
                        name: row.get(1)?,
                    })
                })
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?
                .collect::<Result<Vec<UserProfile>, _>>()
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(profiles)
        })
        .map_err(ApplicationError::from)
    }

    pub async fn delete_profile(
        &self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            tx.execute(
                "DELETE FROM user_profiles WHERE id = ? AND name = ?",
                params![profile.id, profile.name],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    pub async fn list_profiles(
        &self,
    ) -> Result<Vec<UserProfile>, ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT id, name FROM user_profiles ORDER BY created_at \
                     DESC",
                )
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            let profiles = stmt
                .query_map([], |row| {
                    Ok(UserProfile {
                        id: row.get(0)?,
                        name: row.get(1)?,
                    })
                })
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?
                .collect::<Result<Vec<UserProfile>, _>>()
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(profiles)
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_default_profile(
        &self,
    ) -> Result<Option<UserProfile>, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT id, name FROM user_profiles WHERE is_default = 1",
                [],
                |row| {
                    Ok(UserProfile {
                        id: row.get(0)?,
                        name: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|e| DatabaseOperationError::SqliteError(e))
        })
        .map_err(ApplicationError::from)
    }

    pub async fn set_default_profile(
        &self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "UPDATE user_profiles SET is_default = 0 WHERE is_default = 1",
                [],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            tx.execute(
                "UPDATE user_profiles SET is_default = 1 WHERE id = ? AND \
                 name = ?",
                params![profile.id, profile.name],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }
}
