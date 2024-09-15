mod content_operations;
mod encryption_operations;
mod provider_config;
mod user_profile;
use std::path::PathBuf;
use std::sync::Arc;

use lumni::api::error::{ApplicationError, EncryptionError};
pub use provider_config::{ProviderConfig, ProviderConfigOptions};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::Mutex as TokioMutex;

use super::connector::{DatabaseConnector, DatabaseOperationError};
use super::encryption::EncryptionHandler;
use super::{ModelBackend, ModelServer, ModelSpec};
use crate::external as lumni;

#[derive(Debug, Clone)]
pub struct UserProfileDbHandler {
    pub profile: Option<UserProfile>,
    db: Arc<TokioMutex<DatabaseConnector>>,
    encryption_handler: Option<Arc<EncryptionHandler>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserProfile {
    pub id: i64,
    pub name: String,
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

    pub async fn model_backend(
        &mut self,
    ) -> Result<Option<ModelBackend>, ApplicationError> {
        let user_profile = self.profile.clone();

        if let Some(profile) = user_profile {
            self.unlock_profile_settings(&profile).await?;
            let settings = self
                .get_profile_settings(&profile, MaskMode::Unmask)
                .await?;

            let model_server = match settings
                .get("__TEMPLATE.__MODEL_SERVER")
                .and_then(|v| v.as_str())
            {
                Some(server) => server,
                None => return Ok(None),
            };

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

    pub async fn unlock_profile_settings(
        &mut self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        // check if profile is already set
        if let Some(current_profile) = &self.profile {
            if current_profile == profile && self.encryption_handler.is_some() {
                // profile already set
                return Ok(());
            }
        }
        // change profile
        self.profile = Some(profile.clone());
        self.unlock_current_profile().await
    }

    async fn unlock_current_profile(&mut self) -> Result<(), ApplicationError> {
        let profile = if self.profile.is_none() {
            return Err(ApplicationError::InvalidInput(
                "Profile not set".to_string(),
            ));
        } else {
            self.profile.clone().unwrap()
        };

        let (key_hash, key_path): (String, String) = {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                tx.query_row(
                    "SELECT encryption_keys.sha256_hash, \
                     encryption_keys.file_path
                     FROM user_profiles
                     JOIN encryption_keys ON user_profiles.encryption_key_id = \
                     encryption_keys.id
                     WHERE user_profiles.name = ?",
                    params![profile.name],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(DatabaseOperationError::SqliteError)
            })?
        };
        if self.encryption_handler.is_none() {
            // encryption handled required for decryption
            let encryption_handler =
                EncryptionHandler::new_from_path(&PathBuf::from(&key_path))?
                    .ok_or_else(|| {
                        ApplicationError::InvalidInput(
                            "Failed to load encryption handler".to_string(),
                        )
                    })?;

            self.set_encryption_handler(Arc::new(encryption_handler))?;
            self.verify_encryption_key_hash(&key_hash)?;
        }
        Ok(())
    }

    pub async fn export_profile_settings(
        &mut self,
        profile: &UserProfile,
    ) -> Result<JsonValue, ApplicationError> {
        self.unlock_profile_settings(profile).await?;
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
                DELETE FROM provider_configs;
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
