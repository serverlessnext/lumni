mod content_operations;
mod database_operations;
mod encryption_operations;
mod profile_operations;
mod provider_config;
use std::sync::Arc;

use lumni::api::error::{ApplicationError, EncryptionError};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value as JsonValue};
use tokio::sync::Mutex as TokioMutex;

use super::connector::{DatabaseConnector, DatabaseOperationError};
use super::encryption::EncryptionHandler;
use super::{
    AdditionalSetting, ModelBackend, ModelServer, ModelSpec, ProviderConfig,
};
use crate::external as lumni;

#[derive(Debug, Clone, PartialEq)]
pub struct UserProfile {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct UserProfileDbHandler {
    pub profile: Option<UserProfile>,
    db: Arc<TokioMutex<DatabaseConnector>>,
    encryption_handler: Option<Arc<EncryptionHandler>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EncryptionMode {
    Encrypt,
    Decrypt,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MaskMode {
    Mask,
    Unmask,
}

impl UserProfileDbHandler {
    pub fn new(
        profile: Option<UserProfile>,
        db: Arc<TokioMutex<DatabaseConnector>>,
        encryption_handler: Option<Arc<EncryptionHandler>>,
    ) -> Self {
        UserProfileDbHandler {
            profile,
            db,
            encryption_handler,
        }
    }

    pub fn set_profile(&mut self, profile: UserProfile) {
        self.profile = Some(profile);
    }

    pub fn get_profile(&self) -> Option<&UserProfile> {
        self.profile.as_ref()
    }

    pub async fn model_backend(
        &mut self,
    ) -> Result<Option<ModelBackend>, ApplicationError> {
        let user_profile = self.profile.clone();

        if let Some(profile) = user_profile {
            let settings = self
                .get_profile_settings(&profile, MaskMode::Unmask)
                .await?;

            let model_server = settings
                .get("__TEMPLATE.__MODEL_SERVER")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ApplicationError::InvalidInput(
                        "MODEL_SERVER not found in profile".to_string(),
                    )
                })?;

            let server = ModelServer::from_str(model_server)?;

            let model = settings
                .get("__TEMPLATE.MODEL_IDENTIFIER")
                .and_then(|v| v.as_str())
                .map(|identifier| ModelSpec::new_with_validation(identifier))
                .transpose()?;

            Ok(Some(ModelBackend { server, model }))
        } else {
            Ok(None)
        }
    }

    pub fn set_encryption_handler(
        &mut self,
        encryption_handler: Arc<EncryptionHandler>,
    ) -> Result<(), ApplicationError> {
        // If profile is not yet set, return error as we need to know the profile to validate against existing encryption handler
        let profile = self.profile.as_ref().ok_or_else(|| {
            ApplicationError::InvalidInput(
                "UserProfile must be defined before setting encryption handler"
                    .to_string(),
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
                            params![profile.name],
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

    pub fn set_profile_with_encryption_handler(
        &mut self,
        profile: UserProfile,
        encryption_handler: Arc<EncryptionHandler>,
    ) -> Result<(), ApplicationError> {
        self.set_profile(profile);
        self.set_encryption_handler(encryption_handler)
    }

    pub async fn export_profile_settings(
        &mut self,
        profile: &UserProfile,
    ) -> Result<JsonValue, ApplicationError> {
        let settings =
            self.get_profile_settings(profile, MaskMode::Unmask).await?;
        self.create_export_json(&settings).await
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
