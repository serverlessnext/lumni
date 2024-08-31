use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

impl UserProfileDbHandler {
    pub async fn save_provider_config(
        &mut self,
        config: &ProviderConfig,
    ) -> Result<ProviderConfig, ApplicationError> {
        let encryption_key_id = self.get_or_create_encryption_key().await?;
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let additional_settings_json = serde_json::to_string(
                &config.additional_settings,
            )
            .map_err(|e| {
                DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "Failed to serialize additional settings: {}",
                        e
                    )),
                )
            })?;

            let encrypted_value = self
                .encrypt_value(&JsonValue::String(additional_settings_json))
                .map_err(DatabaseOperationError::ApplicationError)?;

            let encrypted_value_json = serde_json::to_string(&encrypted_value)
                .map_err(|e| {
                    DatabaseOperationError::ApplicationError(
                        ApplicationError::InvalidInput(format!(
                            "Failed to serialize encrypted value: {}",
                            e
                        )),
                    )
                })?;

            let config_id = if let Some(id) = config.id {
                // Update existing config
                tx.execute(
                    "UPDATE provider_configs SET 
                     name = ?, provider_type = ?, model_identifier = ?, 
                     additional_settings = ? 
                     WHERE id = ?",
                    params![
                        config.name,
                        config.provider_type,
                        config.model_identifier,
                        encrypted_value_json,
                        id as i64
                    ],
                )?;
                id
            } else {
                // Insert new config
                tx.execute(
                    "INSERT INTO provider_configs 
                     (name, provider_type, model_identifier, 
                     additional_settings, encryption_key_id) 
                     VALUES (?, ?, ?, ?, ?)",
                    params![
                        config.name,
                        config.provider_type,
                        config.model_identifier,
                        encrypted_value_json,
                        encryption_key_id
                    ],
                )?;
                tx.last_insert_rowid()
            };

            Ok(ProviderConfig {
                id: Some(config_id),
                name: config.name.clone(),
                provider_type: config.provider_type.clone(),
                model_identifier: config.model_identifier.clone(),
                additional_settings: config.additional_settings.clone(),
            })
        })
        .map_err(ApplicationError::from)
    }

    pub async fn load_provider_configs(
        &self,
    ) -> Result<Vec<ProviderConfig>, ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let mut stmt = tx.prepare(
                "SELECT pc.id, pc.name, pc.provider_type,
                 pc.model_identifier, pc.additional_settings,
                 ek.file_path as encryption_key_path, ek.sha256_hash
                 FROM provider_configs pc
                 JOIN encryption_keys ek ON pc.encryption_key_id = ek.id",
            )?;
            let configs = stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let name: String = row.get(1)?;
                let provider_type: String = row.get(2)?;
                let model_identifier: Option<String> = row.get(3)?;
                let encrypted_value_json: String = row.get(4)?;
                let encryption_key_path: String = row.get(5)?;
                let sha256_hash: String = row.get(6)?;
                Ok((
                    id,
                    name,
                    provider_type,
                    model_identifier,
                    encrypted_value_json,
                    encryption_key_path,
                    sha256_hash,
                ))
            })?;

            let mut result = Vec::new();
            for config in configs {
                let (
                    id,
                    name,
                    provider_type,
                    model_identifier,
                    encrypted_value_json,
                    encryption_key_path,
                    sha256_hash,
                ) = config?;

                // Create a new EncryptionHandler for this config
                let encryption_handler = EncryptionHandler::new_from_path(&PathBuf::from(&encryption_key_path))
                    .map_err(|e| DatabaseOperationError::ApplicationError(e))?
                    .ok_or_else(|| DatabaseOperationError::ApplicationError(ApplicationError::EncryptionError(
                        EncryptionError::InvalidKey("Failed to create encryption handler".to_string())
                    )))?;

                // Verify the encryption key hash
                if encryption_handler.get_sha256_hash() != sha256_hash {
                    return Err(DatabaseOperationError::ApplicationError(ApplicationError::EncryptionError(
                        EncryptionError::InvalidKey("Encryption key hash mismatch".to_string())
                    )));
                }

                let encrypted_value: JsonValue = serde_json::from_str(&encrypted_value_json)
                    .map_err(|e| DatabaseOperationError::ApplicationError(ApplicationError::InvalidInput(
                        format!("Failed to deserialize encrypted value: {}", e)
                    )))?;

                // Use the encryption handler to decrypt the value
                let decrypted_value = if let (Some(content), Some(encryption_key)) = (
                    encrypted_value["content"].as_str(),
                    encrypted_value["encryption_key"].as_str()
                ) {
                    encryption_handler.decrypt_string(content, encryption_key)
                        .map_err(|e| DatabaseOperationError::ApplicationError(e))?
                } else {
                    return Err(DatabaseOperationError::ApplicationError(
                        ApplicationError::InvalidInput("Invalid encrypted value format".to_string())
                    ));
                };

                let additional_settings: HashMap<String, ProviderConfigOptions> =
                    serde_json::from_str(&decrypted_value)
                        .map_err(|e| DatabaseOperationError::ApplicationError(ApplicationError::InvalidInput(
                            format!("Failed to deserialize decrypted settings: {}", e)
                        )))?;

                result.push(ProviderConfig {
                    id: Some(id),
                    name,
                    provider_type,
                    model_identifier,
                    additional_settings,
                });
            }
            Ok(result)
        })
        .map_err(ApplicationError::from)
    }

    pub async fn delete_provider_config(
        &self,
        config_id: i64,
    ) -> Result<(), ApplicationError> {
        let mut db = self.db.lock().await;
        db.process_queue_with_result(|tx| {
            let deleted_rows = tx.execute(
                "DELETE FROM provider_configs WHERE id = ?",
                params![config_id as i64],
            )?;

            if deleted_rows == 0 {
                Err(DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "No provider config found with ID {}",
                        config_id
                    )),
                ))
            } else {
                Ok(())
            }
        })
        .map_err(ApplicationError::from)
    }
}
