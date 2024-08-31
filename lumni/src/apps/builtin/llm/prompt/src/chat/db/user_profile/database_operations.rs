use std::path::PathBuf;

use lumni::api::error::ApplicationError;
use rusqlite::{params, OptionalExtension};

use super::{
    DatabaseOperationError, EncryptionHandler, UserProfile,
    UserProfileDbHandler,
};
use crate::external as lumni;

impl UserProfileDbHandler {
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
                 FROM user_profiles
                 JOIN encryption_keys ON user_profiles.encryption_key_id = \
                 encryption_keys.id
                 WHERE user_profiles.id = ?",
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
}
