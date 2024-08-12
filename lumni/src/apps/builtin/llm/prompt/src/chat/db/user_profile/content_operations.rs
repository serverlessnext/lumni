use lumni::api::error::ApplicationError;
use serde_json::{json, Map, Value as JsonValue};

use super::{
    DatabaseOperationError, EncryptionMode, MaskMode, UserProfileDbHandler,
};
use crate::external as lumni;

impl UserProfileDbHandler {
    pub fn merge_settings(
        &self,
        current_data: Option<String>,
        new_settings: &JsonValue,
    ) -> Result<JsonValue, DatabaseOperationError> {
        if let Some(current_json) = current_data {
            let current: JsonValue = serde_json::from_str(&current_json)
                .map_err(|e| {
                    DatabaseOperationError::ApplicationError(
                        ApplicationError::InvalidInput(format!(
                            "Invalid JSON: {}",
                            e
                        )),
                    )
                })?;

            let mut merged = current.clone();
            if let (Some(merged_obj), Some(new_obj)) =
                (merged.as_object_mut(), new_settings.as_object())
            {
                for (key, new_value) in new_obj {
                    self.merge_setting(merged_obj, key, new_value, &current)?;
                }
            }
            Ok(merged)
        } else {
            Ok(new_settings.clone())
        }
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
        if is_new_value_marked_for_encryption {
            merged_obj.insert(key.clone(), new_value.clone());
        } else if let Some(content) = new_value.as_str() {
            let encrypted = self
                .encrypt_value(content)
                .map_err(DatabaseOperationError::ApplicationError)?;
            merged_obj.insert(key.clone(), encrypted);
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

    fn process_value(
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
            if let Some(JsonValue::String(content)) = value.get("content") {
                self.encrypt_value(content)
            } else {
                Err(ApplicationError::InvalidInput(
                    "Invalid secure string format".to_string(),
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
                Ok(json!({
                    "value": decrypted,
                    "was_encrypted": true
                }))
            } else {
                Ok(decrypted)
            }
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
