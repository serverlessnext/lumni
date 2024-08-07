use std::sync::Arc;

use rusqlite::{params, Error as SqliteError, OptionalExtension};
use serde_json::Value as JsonValue;
use tokio::sync::Mutex as TokioMutex;

use super::connector::DatabaseConnector;

#[derive(Clone)]
pub struct UserProfileDbHandler {
    profile_name: Option<String>,
    db: Arc<TokioMutex<DatabaseConnector>>,
}

impl UserProfileDbHandler {
    pub fn new(
        profile_name: Option<String>,
        db: Arc<TokioMutex<DatabaseConnector>>,
    ) -> Self {
        UserProfileDbHandler { profile_name, db }
    }

    pub fn get_profile_name(&self) -> Option<&String> {
        self.profile_name.as_ref()
    }

    pub fn set_profile_name(&mut self, profile_name: String) {
        self.profile_name = Some(profile_name);
    }

    pub async fn create_or_update(
        &self,
        profile_name: &str,
        new_settings: &JsonValue,
    ) -> Result<(), SqliteError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            // Check if the profile exists and get current settings
            let current_settings: Option<String> = tx
                .query_row(
                    "SELECT options FROM user_profiles WHERE name = ?",
                    params![profile_name],
                    |row| row.get(0),
                )
                .optional()?;

            let merged_settings = if let Some(current_json) = current_settings {
                // Parse current settings
                let mut current: serde_json::Value =
                    serde_json::from_str(&current_json).map_err(|e| {
                        SqliteError::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                // Merge new settings into current settings
                if let Some(current_obj) = current.as_object_mut() {
                    if let Some(new_obj) = new_settings.as_object() {
                        for (key, value) in new_obj {
                            current_obj.insert(key.clone(), value.clone());
                        }
                    }
                }
                current
            } else {
                // If no current settings, use new settings as is
                new_settings.clone()
            };

            let json_string =
                serde_json::to_string(&merged_settings).map_err(|e| {
                    SqliteError::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            // Insert or replace with merged settings
            tx.execute(
                "INSERT OR REPLACE INTO user_profiles (name, options) VALUES \
                 (?, ?)",
                params![profile_name, json_string],
            )?;

            Ok(())
        })
    }

    pub async fn get_profile_settings(
        &self,
        profile_name: &str,
    ) -> Result<JsonValue, SqliteError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let json_string: String = tx.query_row(
                "SELECT options FROM user_profiles WHERE name = ?",
                params![profile_name],
                |row| row.get(0),
            )?;
            serde_json::from_str(&json_string).map_err(|e| {
                SqliteError::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })
    }

    pub async fn delete_profile(
        &self,
        profile_name: &str,
    ) -> Result<(), SqliteError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "DELETE FROM user_profiles WHERE name = ?",
                params![profile_name],
            )?;
            Ok(())
        })
    }

    pub async fn set_default_profile(
        &self,
        profile_name: &str,
    ) -> Result<(), SqliteError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES \
                 ('default_profile', ?)",
                params![profile_name],
            )?;
            Ok(())
        })
    }

    pub async fn get_default_profile(
        &self,
    ) -> Result<Option<String>, SqliteError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT value FROM metadata WHERE key = 'default_profile'",
                [],
                |row| row.get(0),
            )
            .optional()
        })
    }

    pub async fn list_profiles(&self) -> Result<Vec<String>, SqliteError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let mut stmt = tx.prepare("SELECT name FROM user_profiles")?;
            let profiles = stmt.query_map([], |row| row.get(0))?;
            profiles.collect()
        })
    }
}
