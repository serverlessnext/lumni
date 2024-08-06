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

    pub async fn set_profile_settings(
        &self,
        profile_name: &str,
        settings: &JsonValue,
    ) -> Result<(), SqliteError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let json_string = serde_json::to_string(settings).map_err(|e| {
                SqliteError::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
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

    pub async fn update_profile_setting(
        &self,
        profile_name: &str,
        key: &str,
        value: &JsonValue,
    ) -> Result<(), SqliteError> {
        let mut settings = self.get_profile_settings(profile_name).await?;
        if let Some(obj) = settings.as_object_mut() {
            obj.insert(key.to_string(), value.clone());
            self.set_profile_settings(profile_name, &settings).await?;
        }
        Ok(())
    }
}
