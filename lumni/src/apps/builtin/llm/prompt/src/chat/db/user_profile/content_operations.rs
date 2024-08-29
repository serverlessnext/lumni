use lumni::api::error::ApplicationError;
use rusqlite::params;
use serde_json::{json, Map, Value as JsonValue};

use super::{
    DatabaseOperationError, EncryptionMode, MaskMode, UserProfile,
    UserProfileDbHandler,
};
use crate::external as lumni;

impl UserProfileDbHandler {
    pub async fn update_profile_settings(
        &mut self,
        profile: &UserProfile,
        new_settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        // Retrieve existing settings and merge with new settings
        let existing_settings =
            self.get_profile_settings(profile, MaskMode::Unmask).await?;
        let merged_settings =
            self.merge_settings(&existing_settings, new_settings)?;

        let processed_settings = self.process_settings(
            &merged_settings,
            EncryptionMode::Encrypt,
            MaskMode::Unmask,
        )?;

        // Serialize the processed settings
        let json_string =
            serde_json::to_string(&processed_settings).map_err(|e| {
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
                    "UPDATE user_profiles SET options = ? WHERE id = ?",
                    params![json_string, profile.id],
                )
                .map_err(DatabaseOperationError::SqliteError)?;

            if updated_rows == 0 {
                return Err(DatabaseOperationError::ApplicationError(
                    ApplicationError::InvalidInput(format!(
                        "Profile with id {} not found",
                        profile.id
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

    pub async fn create_export_json(
        &self,
        settings: &JsonValue,
    ) -> Result<JsonValue, ApplicationError> {
        let (key_path, key_sha256) = self.get_encryption_key_info().await?;

        let export = match settings {
            JsonValue::Object(obj) => {
                let parameters = obj
                    .iter()
                    .map(|(key, value)| {
                        let processed_value = self
                            .process_value_with_metadata(
                                value,
                                EncryptionMode::Decrypt,
                                MaskMode::Unmask,
                            )
                            .unwrap_or_else(|_| value.clone());

                        let (param_type, param_value, encrypted) =
                            if let Some(metadata) = processed_value.as_object()
                            {
                                if metadata.get("was_encrypted")
                                    == Some(&JsonValue::Bool(true))
                                {
                                    (
                                        metadata
                                            .get("type")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("Unknown"),
                                        metadata
                                            .get("content")
                                            .unwrap_or(value)
                                            .clone(),
                                        true,
                                    )
                                } else {
                                    self.get_parameter_info(&processed_value)
                                }
                            } else {
                                self.get_parameter_info(&processed_value)
                            };

                        json!({
                            "Key": key,
                            "Value": param_value,
                            "Type": param_type,
                            "Encrypted": encrypted
                        })
                    })
                    .collect::<Vec<JsonValue>>();

                json!({
                    "Parameters": parameters,
                    "EncryptionKey": {
                        "Path": key_path,
                        "SHA256": key_sha256
                    }
                })
            }
            _ => JsonValue::Null,
        };
        Ok(export)
    }

    fn get_parameter_info(
        &self,
        value: &JsonValue,
    ) -> (&'static str, JsonValue, bool) {
        match value {
            JsonValue::Null => ("Null", value.clone(), false),
            JsonValue::Bool(_) => ("Boolean", value.clone(), false),
            JsonValue::Number(_) => ("Number", value.clone(), false),
            JsonValue::String(_) => ("String", value.clone(), false),
            JsonValue::Array(_) => ("Array", value.clone(), false),
            JsonValue::Object(_) => ("Object", value.clone(), false),
        }
    }

    fn get_json_type(&self, value: &JsonValue) -> (&'static str, JsonValue) {
        match value {
            JsonValue::Null => ("Null", JsonValue::Null),
            JsonValue::Bool(_) => ("Boolean", value.clone()),
            JsonValue::Number(_) => ("Number", value.clone()),
            JsonValue::String(_) => ("String", value.clone()),
            JsonValue::Array(_) => ("Array", value.clone()),
            JsonValue::Object(_) => ("Object", value.clone()),
        }
    }

    fn process_value_with_metadata(
        &self,
        value: &JsonValue,
        encryption_mode: EncryptionMode,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        if encryption_mode == EncryptionMode::Encrypt {
            self.handle_encryption(value)
        } else {
            let decrypted = self.handle_decryption(value, mask_mode)?;
            if Self::is_encrypted_value(value) {
                let (param_type, _) = self.get_json_type(&decrypted);
                Ok(json!({
                    "content": decrypted,
                    "was_encrypted": true,
                    "type": param_type
                }))
            } else {
                Ok(decrypted)
            }
        }
    }

    fn merge_settings(
        &self,
        current_data: &JsonValue,
        new_settings: &JsonValue,
    ) -> Result<JsonValue, DatabaseOperationError> {
        let mut merged = current_data.clone();
        if let (Some(merged_obj), Some(new_obj)) =
            (merged.as_object_mut(), new_settings.as_object())
        {
            for (key, new_value) in new_obj {
                self.merge_setting(merged_obj, key, new_value, current_data)?;
            }
        }
        Ok(merged)
    }

    fn merge_setting(
        &self,
        merged_obj: &mut Map<String, JsonValue>,
        key: &String,
        new_value: &JsonValue,
        current: &JsonValue,
    ) -> Result<(), DatabaseOperationError> {
        if new_value.is_null() {
            merged_obj.remove(key);
        } else {
            let current_value = current.get(key);
            let is_currently_encrypted =
                current_value.map(Self::is_encrypted_value).unwrap_or(false);
            let is_new_value_marked_for_encryption =
                Self::is_marked_for_encryption(new_value);

            if is_currently_encrypted {
                self.handle_encrypted_value(
                    merged_obj,
                    key,
                    new_value,
                    is_new_value_marked_for_encryption,
                )?;
            } else if is_new_value_marked_for_encryption {
                merged_obj.insert(key.clone(), new_value.clone());
            } else {
                merged_obj.insert(key.clone(), new_value.clone());
            }
        }
        Ok(())
    }

    fn handle_encrypted_value(
        &self,
        merged_obj: &mut Map<String, JsonValue>,
        key: &String,
        new_value: &JsonValue,
        is_new_value_marked_for_encryption: bool,
    ) -> Result<(), DatabaseOperationError> {
        // Check if the existing value is already encrypted
        let is_existing_value_encrypted = merged_obj
            .get(key)
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("encryption_key"))
            .is_some();

        if is_new_value_marked_for_encryption || is_existing_value_encrypted {
            // If the new value is marked for encryption or the existing value was encrypted,
            // we need to encrypt the new value
            let encrypted_value = self
                .encrypt_value(new_value)
                .map_err(DatabaseOperationError::ApplicationError)?;

            // Insert the encrypted value
            merged_obj.insert(key.clone(), encrypted_value);
        } else {
            // If it's not marked for encryption and the existing value wasn't encrypted,
            // we can insert it as is
            merged_obj.insert(key.clone(), new_value.clone());
        }
        Ok(())
    }

    pub fn process_settings(
        &self,
        value: &JsonValue,
        encryption_mode: EncryptionMode,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        match value {
            JsonValue::Object(obj) => {
                let mut new_obj = Map::new();
                for (k, v) in obj {
                    new_obj.insert(
                        k.clone(),
                        self.process_value(v, encryption_mode, mask_mode)?,
                    );
                }
                Ok(JsonValue::Object(new_obj))
            }
            JsonValue::Array(arr) => {
                let new_arr: Result<Vec<JsonValue>, _> = arr
                    .iter()
                    .map(|v| {
                        self.process_settings(v, encryption_mode, mask_mode)
                    })
                    .collect();
                Ok(JsonValue::Array(new_arr?))
            }
            _ => Ok(value.clone()),
        }
    }

    pub fn process_value(
        &self,
        value: &JsonValue,
        encryption_mode: EncryptionMode,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        if encryption_mode == EncryptionMode::Encrypt {
            self.handle_encryption(value)
        } else {
            self.handle_decryption(value, mask_mode)
        }
    }

    fn handle_encryption(
        &self,
        value: &JsonValue,
    ) -> Result<JsonValue, ApplicationError> {
        if Self::is_marked_for_encryption(value) {
            if let Some(content) = value.get("content") {
                let encrypted_value = self.encrypt_value(content)?;

                // Check if the encrypted_value already has the correct structure
                if encrypted_value.is_object()
                    && encrypted_value.get("content").is_some()
                    && encrypted_value.get("encryption_key").is_some()
                    && encrypted_value.get("type_info").is_some()
                {
                    Ok(encrypted_value)
                } else {
                    // If not, construct the correct structure
                    Ok(json!({
                        "content": encrypted_value.get("content").unwrap_or(&JsonValue::Null),
                        "encryption_key": encrypted_value.get("encryption_key").unwrap_or(&JsonValue::String("".to_string())),
                        "type_info": encrypted_value.get("type_info").unwrap_or(&JsonValue::String("unknown".to_string()))
                    }))
                }
            } else {
                Err(ApplicationError::InvalidInput(
                    "Invalid secure value format".to_string(),
                ))
            }
        } else {
            Ok(value.clone())
        }
    }

    fn handle_decryption(
        &self,
        value: &JsonValue,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        if Self::is_encrypted_value(value) {
            if self.encryption_handler.is_some() {
                if mask_mode == MaskMode::Mask {
                    Ok(JsonValue::String("*****".to_string()))
                } else {
                    self.decrypt_value(value)
                }
            } else {
                Ok(JsonValue::String("*****".to_string())) // Always mask if no encryption handler
            }
        } else {
            Ok(value.clone())
        }
    }

    pub fn process_settings_with_metadata(
        &self,
        value: &JsonValue,
        encryption_mode: EncryptionMode,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        match value {
            JsonValue::Object(obj) => {
                let mut new_obj = Map::new();
                for (k, v) in obj {
                    let processed = self.process_value_with_metadata(
                        v,
                        encryption_mode,
                        mask_mode,
                    )?;
                    new_obj.insert(k.clone(), processed);
                }
                Ok(JsonValue::Object(new_obj))
            }
            JsonValue::Array(arr) => {
                let new_arr: Result<Vec<JsonValue>, _> = arr
                    .iter()
                    .map(|v| {
                        self.process_settings_with_metadata(
                            v,
                            encryption_mode,
                            mask_mode,
                        )
                    })
                    .collect();
                Ok(JsonValue::Array(new_arr?))
            }
            _ => Ok(value.clone()),
        }
    }

    pub fn is_encrypted_value(value: &JsonValue) -> bool {
        if let Some(obj) = value.as_object() {
            obj.contains_key("content") && obj.contains_key("encryption_key")
        } else {
            false
        }
    }

    pub fn is_marked_for_encryption(value: &JsonValue) -> bool {
        if let Some(obj) = value.as_object() {
            obj.contains_key("content")
                && obj.get("encryption_key")
                    == Some(&JsonValue::String("".to_string()))
        } else {
            false
        }
    }
}
