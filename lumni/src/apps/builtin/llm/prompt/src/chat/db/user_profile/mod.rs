mod content_operations;
mod database_operations;
mod encryption_operations;
mod profile_operations;
use std::sync::Arc;

use lumni::api::error::ApplicationError;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value as JsonValue};
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

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum EncryptionMode {
    Encrypt,
    Decrypt,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum MaskMode {
    Mask,
    Unmask,
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

    pub fn set_profile_name(&mut self, profile_name: String) {
        self.profile_name = Some(profile_name);
    }

    pub fn set_encryption_handler(
        &mut self,
        encryption_handler: Arc<EncryptionHandler>,
    ) -> Result<(), ApplicationError> {
        // If profile is not yet set, return error as we need to know the profile to validate against existing encryption handler
        let profile_name = self.profile_name.as_ref().ok_or_else(|| {
            ApplicationError::InvalidInput(
                "Profile name must be defined before setting encryption handler".to_string(),
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
                             FROM user_profiles
                             JOIN encryption_keys ON \
                             user_profiles.encryption_key_id = \
                             encryption_keys.id
                             WHERE user_profiles.name = ?",
                            params![profile_name],
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

    pub async fn export_profile_settings(
        &mut self,
        profile_name: &str,
    ) -> Result<JsonValue, ApplicationError> {
        let settings = self
            .get_profile_settings(profile_name, MaskMode::Unmask)
            .await?;
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

    pub async fn truncate_and_vacuum(&self) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            // Disable foreign key constraints temporarily
            tx.execute("PRAGMA foreign_keys = OFF", [])?;

            // NOTE: encryption_keys is currently only used in user_profiles, if this changes (e.g. keys used to encrypt conversations) we should add a check so we only delete unused keys -- for now, just delete everything
            tx.execute_batch(
                "
                DELETE FROM user_profiles;
                DELETE FROM encryption_keys;
            ",
            )?;

            // Re-enable foreign key constraints
            tx.execute("PRAGMA foreign_keys = ON", [])?;

            Ok(())
        })?;

        // Vacuum the database to reclaim unused space
        db.vacuum()?;
        Ok(())
    }
}
