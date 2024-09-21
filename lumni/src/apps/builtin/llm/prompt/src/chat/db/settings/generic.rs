use lumni::api::error::ApplicationError;
use lumni::Timestamp;
use rusqlite::{params, OptionalExtension};
use serde_json::Value as JsonValue;

use super::{
    DatabaseOperationError, EncryptionMode, MaskMode, UserProfile,
    UserProfileDbHandler,
};
use crate::external as lumni;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DatabaseConfigurationItem {
    pub id: i64,
    pub name: String,
    pub section: String,
}

impl UserProfileDbHandler {
    pub async fn create_configuration_item(
        &mut self,
        name: String,
        section: &str,
        parameters: JsonValue,
    ) -> Result<DatabaseConfigurationItem, ApplicationError> {
        let timestamp = Timestamp::from_system_time().unwrap().as_millis();

        let encryption_key_id = self.get_or_create_encryption_key().await?;
        let processed_parameters = self.process_parameters(
            &parameters,
            EncryptionMode::Encrypt,
            MaskMode::Unmask,
        )?;

        let json_string = serde_json::to_string(&processed_parameters)
            .map_err(|e| {
                ApplicationError::InvalidInput(format!(
                    "Failed to serialize JSON: {}",
                    e
                ))
            })?;

        let mut db = self.db.lock().await;
        let item = db
            .process_queue_with_result(|tx| {
                tx.execute(
                    "INSERT INTO configuration (name, section, parameters, \
                     encryption_key_id, created_at) VALUES (?, ?, ?, ?, ?)",
                    params![
                        name,
                        section,
                        json_string,
                        encryption_key_id,
                        timestamp
                    ],
                )
                .map_err(DatabaseOperationError::SqliteError)?;

                let id = tx.last_insert_rowid();
                Ok(DatabaseConfigurationItem {
                    id,
                    name: name.to_string(),
                    section: section.to_string(),
                })
            })
            .map_err(|e| match e {
                DatabaseOperationError::SqliteError(sqlite_err) => {
                    ApplicationError::DatabaseError(sqlite_err.to_string())
                }
                DatabaseOperationError::ApplicationError(app_err) => app_err,
            })?;

        Ok(item)
    }

    pub async fn update_configuration_item(
        &mut self,
        item: &DatabaseConfigurationItem,
        new_parameters: &JsonValue,
    ) -> Result<(), ApplicationError> {
        // Update configuration parameters
        self.update_configuration_parameters(item, new_parameters)
            .await?;

        // Update the name if it has changed
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "UPDATE configuration SET name = ? WHERE id = ? AND section = \
                 ?",
                params![item.name, item.id, item.section],
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

        Ok(())
    }

    pub async fn rename_configuration_item(
        &self,
        item: &DatabaseConfigurationItem,
        new_name: &str,
    ) -> Result<(), ApplicationError> {
        log::debug!(
            "Renaming configuration item '{}' (ID: {}) to '{}'",
            item.name,
            item.id,
            new_name
        );
        if item.name == new_name {
            return Ok(()); // No need to rename if the names are the same
        }

        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            // Perform the rename
            let updated_rows = tx.execute(
                "UPDATE configuration SET name = ? WHERE id = ? AND section = \
                 ?",
                params![new_name, item.id, item.section],
            )?;

            if updated_rows == 0 {
                Err(DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "Configuration item '{}' (ID: {}) not found",
                        item.name, item.id
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

    async fn update_configuration_parameters(
        &mut self,
        item: &DatabaseConfigurationItem,
        new_parameters: &JsonValue,
    ) -> Result<(), ApplicationError> {
        // Retrieve existing parameters and merge with new parameters
        let existing_parameters = self
            .get_configuration_parameters(item, MaskMode::Unmask)
            .await?;
        let merged_parameters =
            self.merge_parameters(&existing_parameters, new_parameters)?;
        let processed_parameters = self.process_parameters(
            &merged_parameters,
            EncryptionMode::Encrypt,
            MaskMode::Unmask,
        )?;

        // Serialize the processed parameters
        let json_string = serde_json::to_string(&processed_parameters)
            .map_err(|e| {
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
                    "UPDATE configuration SET parameters = ? WHERE id = ? AND \
                     section = ?",
                    params![json_string, item.id, item.section],
                )
                .map_err(DatabaseOperationError::SqliteError)?;

            if updated_rows == 0 {
                return Err(DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "Configuration item with id {} not found",
                        item.id
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

    pub async fn get_configuration_item_by_id(
        &self,
        id: i64,
    ) -> Result<Option<DatabaseConfigurationItem>, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT id, name, section FROM configuration WHERE id = ?",
                params![id],
                |row| {
                    Ok(DatabaseConfigurationItem {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        section: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(|e| DatabaseOperationError::SqliteError(e))
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_configuration_items_by_name(
        &self,
        name: &str,
        section: &str,
    ) -> Result<Vec<DatabaseConfigurationItem>, ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT id, name, section FROM configuration WHERE name = \
                     ? AND section = ?",
                )
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            let items = stmt
                .query_map(params![name, section], |row| {
                    Ok(DatabaseConfigurationItem {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        section: row.get(2)?,
                    })
                })
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?
                .collect::<Result<Vec<DatabaseConfigurationItem>, _>>()
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(items)
        })
        .map_err(ApplicationError::from)
    }

    pub async fn delete_configuration_item(
        &self,
        item: &DatabaseConfigurationItem,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            tx.execute(
                "DELETE FROM configuration WHERE id = ? AND name = ? AND \
                 section = ?",
                params![item.id, item.name, item.section],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    pub async fn list_configuration_items(
        &self,
        section: &str,
    ) -> Result<Vec<DatabaseConfigurationItem>, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT id, name, section FROM configuration WHERE \
                     section = ? ORDER BY created_at DESC",
                )
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            let items = stmt
                .query_map(params![section], |row| {
                    Ok(DatabaseConfigurationItem {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        section: row.get(2)?,
                    })
                })
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?
                .collect::<Result<Vec<DatabaseConfigurationItem>, _>>()
                .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(items)
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_default_configuration_item(
        &self,
        section: &str,
    ) -> Result<Option<DatabaseConfigurationItem>, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.query_row(
                "SELECT id, name, section FROM configuration WHERE section = \
                 ? AND is_default = 1",
                params![section],
                |row| {
                    Ok(DatabaseConfigurationItem {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        section: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(|e| DatabaseOperationError::SqliteError(e))
        })
        .map_err(ApplicationError::from)
    }

    pub async fn set_default_configuration_item(
        &self,
        item: &DatabaseConfigurationItem,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            tx.execute(
                "UPDATE configuration SET is_default = 0 WHERE section = ? \
                 AND is_default = 1",
                params![item.section],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            tx.execute(
                "UPDATE configuration SET is_default = 1 WHERE id = ? AND \
                 name = ? AND section = ?",
                params![item.id, item.name, item.section],
            )
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
            Ok(())
        })
        .map_err(ApplicationError::from)
    }

    pub async fn get_configuration_parameters(
        &self,
        item: &DatabaseConfigurationItem,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        log::debug!(
            "Getting parameters for configuration item: {}:{} ({:?})",
            item.id,
            item.name,
            mask_mode
        );
        let json_string: String = {
            let mut db = self.db.lock().await;
            db.process_queue_with_result(|tx| {
                tx.query_row(
                    "SELECT parameters FROM configuration
                     WHERE section = ? AND name = ?",
                    params![item.section, item.name],
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
        let parameters: JsonValue = serde_json::from_str(&json_string)
            .map_err(|e| {
                ApplicationError::InvalidInput(format!("Invalid JSON: {}", e))
            })?;
        self.process_parameters_with_metadata(
            &parameters,
            EncryptionMode::Decrypt,
            mask_mode,
        )
    }
}

impl From<UserProfile> for DatabaseConfigurationItem {
    fn from(profile: UserProfile) -> Self {
        DatabaseConfigurationItem {
            id: profile.id,
            name: profile.name,
            section: "profile".to_string(),
        }
    }
}

impl From<&UserProfile> for DatabaseConfigurationItem {
    fn from(profile: &UserProfile) -> Self {
        DatabaseConfigurationItem {
            id: profile.id,
            name: profile.name.clone(),
            section: "profile".to_string(),
        }
    }
}
