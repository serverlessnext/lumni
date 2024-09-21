use lumni::api::error::ApplicationError;
use serde_json::{json, Map, Value as JsonValue};

use super::{
    DatabaseOperationError, EncryptionMode, MaskMode, UserProfileDbHandler,
};
use crate::external as lumni;

impl UserProfileDbHandler {
    pub fn type_info(&self, content: &JsonValue) -> &'static str {
        match content {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        }
    }

    pub fn process_parameters(
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
                        self.process_parameters(v, encryption_mode, mask_mode)
                    })
                    .collect();
                Ok(JsonValue::Array(new_arr?))
            }
            _ => Ok(value.clone()),
        }
    }

    pub fn process_parameters_with_metadata(
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
                        self.process_parameters_with_metadata(
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

    pub fn merge_parameters(
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
                                // check if the value was encrypted
                                if metadata.contains_key("__encryption_key") {
                                    (
                                        metadata
                                            .get("__type_info")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("Unknown"),
                                        metadata
                                            .get("__content")
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
                    "__content": decrypted,
                    "__encryption_key": "",
                    "__type_info": param_type
                }))
            } else {
                Ok(decrypted)
            }
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

            let new_value = if is_currently_encrypted
                && !is_new_value_marked_for_encryption
            {
                // current encrypted value exist - ensure new value is also encrypted
                // by adding encryption key marker
                let type_info = self.type_info(new_value);
                json! ({
                    "__content": new_value,
                    "__encryption_key": "",
                    "__type_info":  type_info
                })
            } else {
                new_value.clone()
            };
            merged_obj.insert(key.clone(), new_value);
        }
        Ok(())
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
            if let Some(content) = value.get("__content") {
                let encrypted_value = self.encrypt_value(content)?;

                // Check if the encrypted_value already has the correct structure
                if encrypted_value.is_object()
                    && encrypted_value.get("__content").is_some()
                    && encrypted_value.get("__encryption_key").is_some()
                    && encrypted_value.get("__type_info").is_some()
                {
                    Ok(encrypted_value)
                } else {
                    // If not, construct the correct structure
                    Ok(json!({
                        "__content": encrypted_value.get("__content").unwrap_or(&JsonValue::Null),
                        "__encryption_key": encrypted_value.get("__encryption_key").unwrap_or(&JsonValue::String("".to_string())),
                        "__type_info": encrypted_value.get("__type_info").unwrap_or(&JsonValue::String("unknown".to_string()))
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

    fn is_encrypted_value(value: &JsonValue) -> bool {
        if let Some(obj) = value.as_object() {
            obj.contains_key("__content")
                && obj.contains_key("__encryption_key")
        } else {
            false
        }
    }

    fn is_marked_for_encryption(value: &JsonValue) -> bool {
        if let Some(obj) = value.as_object() {
            obj.contains_key("__content")
                && obj.get("__encryption_key")
                    == Some(&JsonValue::String("".to_string()))
        } else {
            false
        }
    }
}
