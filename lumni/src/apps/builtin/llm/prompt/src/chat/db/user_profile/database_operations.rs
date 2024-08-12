use std::path::PathBuf;

use lumni::api::error::{ApplicationError, EncryptionError};
use rusqlite::{params, OptionalExtension};

use super::{DatabaseOperationError, EncryptionHandler, UserProfileDbHandler};
use crate::external as lumni;

impl UserProfileDbHandler {
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

    pub async fn load_encryption_handler(
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

        EncryptionHandler::new_from_path(&PathBuf::from(key_path))
            .map_err(|e| {
                ApplicationError::EncryptionError(EncryptionError::InvalidKey(
                    e.to_string(),
                ))
            })?
            .ok_or_else(|| {
                ApplicationError::EncryptionError(EncryptionError::InvalidKey(
                    "Failed to create encryption handler from key file"
                        .to_string(),
                ))
            })
    }

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
}
