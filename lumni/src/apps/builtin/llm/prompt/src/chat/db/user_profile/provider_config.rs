use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

impl UserProfileDbHandler {
    pub async fn save_provider_config(
        &mut self,
        config: &ProviderConfig,
    ) -> Result<(), ApplicationError> {
        let encryption_key_id = self.get_or_create_encryption_key().await?;

        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            let additional_settings: HashMap<String, String> = config
                .additional_settings
                .iter()
                .map(|(k, v)| (k.clone(), v.value.clone()))
                .collect();

            let additional_settings_json =
                serde_json::to_string(&additional_settings).map_err(|e| {
                    DatabaseOperationError::ApplicationError(
                        ApplicationError::InvalidInput(format!(
                            "Failed to serialize additional settings: {}",
                            e
                        )),
                    )
                })?;

            // Use encrypt_value method to encrypt the settings
            let encrypted_value = self
                .encrypt_value(&JsonValue::String(additional_settings_json))
                .map_err(DatabaseOperationError::ApplicationError)?;

            // Extract the encrypted content
            let encrypted_settings =
                encrypted_value["content"].as_str().ok_or_else(|| {
                    DatabaseOperationError::ApplicationError(
                        ApplicationError::InvalidInput(
                            "Failed to extract encrypted content".to_string(),
                        ),
                    )
                })?;

            if let Some(id) = config.id {
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
                        encrypted_settings,
                        id as i64
                    ],
                )?;
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
                        encrypted_settings,
                        encryption_key_id
                    ],
                )?;
            }

            Ok(())
        })
        .map_err(ApplicationError::from)
    }
}

impl UserProfileDbHandler {
    pub async fn load_provider_configs(
        &self,
    ) -> Result<Vec<ProviderConfig>, ApplicationError> {
        let mut db = self.db.lock().await;

        db.process_queue_with_result(|tx| {
            let mut stmt = tx.prepare(
                "SELECT pc.id, pc.name, pc.provider_type, \
                 pc.model_identifier, pc.additional_settings, 
                        ek.file_path as encryption_key_path
                 FROM provider_configs pc
                 JOIN encryption_keys ek ON pc.encryption_key_id = ek.id",
            )?;

            let configs = stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let name: String = row.get(1)?;
                let provider_type: String = row.get(2)?;
                let model_identifier: Option<String> = row.get(3)?;
                let additional_settings_encrypted: String = row.get(4)?;
                let encryption_key_path: String = row.get(5)?;

                Ok((
                    id,
                    name,
                    provider_type,
                    model_identifier,
                    additional_settings_encrypted,
                    encryption_key_path,
                ))
            })?;

            let mut result = Vec::new();

            for config in configs {
                let (
                    id,
                    name,
                    provider_type,
                    model_identifier,
                    additional_settings_encrypted,
                    encryption_key_path,
                ) = config?;

                // Load the specific encryption handler for this config
                let encryption_handler = EncryptionHandler::new_from_path(
                    &PathBuf::from(encryption_key_path),
                )
                .map_err(|e| {
                    DatabaseOperationError::ApplicationError(
                        ApplicationError::EncryptionError(
                            EncryptionError::KeyGenerationFailed(e.to_string()),
                        ),
                    )
                })?
                .ok_or_else(|| {
                    DatabaseOperationError::ApplicationError(
                        ApplicationError::EncryptionError(
                            EncryptionError::InvalidKey(
                                "Failed to create encryption handler"
                                    .to_string(),
                            ),
                        ),
                    )
                })?;

                // Decrypt the additional settings using the specific encryption handler
                let decrypted_value = encryption_handler
                    .decrypt_string(
                        &additional_settings_encrypted,
                        "", // The actual key should be retrieved from the encryption handler
                    )
                    .map_err(|e| {
                        DatabaseOperationError::ApplicationError(
                            ApplicationError::EncryptionError(
                                EncryptionError::DecryptionFailed(
                                    e.to_string(),
                                ),
                            ),
                        )
                    })?;

                let additional_settings: HashMap<String, AdditionalSetting> =
                    serde_json::from_str(&decrypted_value).map_err(|e| {
                        DatabaseOperationError::ApplicationError(
                            ApplicationError::InvalidInput(format!(
                                "Failed to deserialize decrypted settings: {}",
                                e
                            )),
                        )
                    })?;

                result.push(ProviderConfig {
                    id: Some(id as usize),
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
}
