mod encryption_operations;
mod profile_operations;
mod content_operations;
mod database_operations;
use std::sync::Arc;

use lumni::api::error::ApplicationError;
use serde_json::{json, Value as JsonValue};
use tokio::sync::Mutex as TokioMutex;

use super::connector::{DatabaseConnector, DatabaseOperationError};
use super::encryption::EncryptionHandler;
use crate::external as lumni;

#[derive(Debug)]
pub struct UserProfileDbHandler {
    profile_name: Option<String>,
    db: Arc<TokioMutex<DatabaseConnector>>,
    encryption_handler: Option<Arc<EncryptionHandler>>,
}

impl UserProfileDbHandler {
    pub fn new(
        profile_name: Option<String>,
        db: Arc<TokioMutex<DatabaseConnector>>,
        encryption_handler: Option<Arc<EncryptionHandler>>,
    ) -> Self {
        UserProfileDbHandler {
            profile_name,
            db,
            encryption_handler,
        }
    }

    pub fn get_profile_name(&self) -> Option<&String> {
        self.profile_name.as_ref()
    }

    pub fn set_profile_name(&mut self, profile_name: String) {
        self.profile_name = Some(profile_name);
    }

    pub fn get_encryption_handler(&self) -> Option<&Arc<EncryptionHandler>> {
        self.encryption_handler.as_ref()
    }

    pub fn set_encryption_handler(
        &mut self,
        encryption_handler: Arc<EncryptionHandler>,
    ) {
        self.encryption_handler = Some(encryption_handler);
    }

    pub async fn export_profile_settings(
        &mut self,
        profile_name: &str,
    ) -> Result<JsonValue, ApplicationError> {
        let settings = self.get_profile_settings(profile_name, false).await?;
        Ok(self.create_export_json(&settings))
    }

    fn create_export_json(&self, settings: &JsonValue) -> JsonValue {
        match settings {
            JsonValue::Object(obj) => {
                let mut parameters = Vec::new();
                for (key, value) in obj {
                    let (param_type, param_value) = if let Some(metadata) =
                        value.as_object()
                    {
                        if metadata.get("was_encrypted")
                            == Some(&JsonValue::Bool(true))
                        {
                            (
                                "SecureString",
                                metadata.get("value").unwrap_or(value).clone(),
                            )
                        } else {
                            ("String", value.clone())
                        }
                    } else {
                        ("String", value.clone())
                    };
                    parameters.push(json!({
                        "Key": key,
                        "Value": param_value,
                        "Type": param_type
                    }));
                }
                json!({
                    "Parameters": parameters
                })
            }
            _ => JsonValue::Null,
        }
    }
}
