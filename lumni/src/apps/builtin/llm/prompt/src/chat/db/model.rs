use lazy_static::lazy_static;
use lumni::api::error::ApplicationError;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub use crate::external as lumni;

lazy_static! {
    static ref IDENTIFIER_REGEX: Regex =
        Regex::new(r"^[-a-z0-9_]+::[-a-z0-9_][-a-z0-9_:.]*[-a-z0-9_]+$")
            .unwrap();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelIdentifier(pub String);

impl ModelIdentifier {
    pub fn new(identifier_str: &str) -> Result<Self, ApplicationError> {
        if IDENTIFIER_REGEX.is_match(identifier_str) {
            Ok(ModelIdentifier(identifier_str.to_string()))
        } else {
            Err(ApplicationError::InvalidInput(format!(
                "Identifier must be in the format 'provider::model_name', \
                 where the provider contains only lowercase letters, numbers, \
                 hyphens, underscores, and the model name can include \
                 internal colons but not start or end with them. Got: '{}'",
                identifier_str
            )))
        }
    }

    pub fn get_model_provider(&self) -> &str {
        // model provider is the first part of the identifier
        self.0.split("::").next().unwrap()
    }

    pub fn get_model_name(&self) -> &str {
        // model name is the second part of the identifier
        self.0.split("::").nth(1).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSpec {
    pub identifier: ModelIdentifier,
    pub info: Option<serde_json::Value>,
    pub config: Option<serde_json::Value>,
    pub context_window_size: Option<i64>,
    pub input_token_limit: Option<i64>,
}

impl ModelSpec {
    pub fn new_with_validation(
        identifier_str: &str,
    ) -> Result<Self, ApplicationError> {
        let identifier = ModelIdentifier::new(identifier_str)?;
        Ok(ModelSpec {
            identifier,
            info: None,
            config: None,
            context_window_size: None,
            input_token_limit: None,
        })
    }

    pub fn get_model_provider(&self) -> &str {
        self.identifier.get_model_provider()
    }

    pub fn get_model_name(&self) -> &str {
        self.identifier.get_model_name()
    }

    pub fn identifier(&self) -> &ModelIdentifier {
        &self.identifier
    }

    pub fn info(&self) -> Option<&serde_json::Value> {
        self.info.as_ref()
    }

    pub fn config(&self) -> Option<&serde_json::Value> {
        self.config.as_ref()
    }

    pub fn context_window_size(&self) -> Option<i64> {
        self.context_window_size
    }

    pub fn input_token_limit(&self) -> Option<i64> {
        self.input_token_limit
    }

    pub fn set_info(&mut self, info: serde_json::Value) -> &mut Self {
        self.info = Some(info);
        self
    }

    pub fn set_config(&mut self, config: serde_json::Value) -> &mut Self {
        self.config = Some(config);
        self
    }

    pub fn set_context_window_size(&mut self, size: i64) -> &mut Self {
        self.context_window_size = Some(size);
        self
    }

    pub fn set_input_token_limit(&mut self, limit: i64) -> &mut Self {
        self.input_token_limit = Some(limit);
        self
    }

    pub fn set_config_value(
        &mut self,
        key: &str,
        value: serde_json::Value,
    ) -> &mut Self {
        if let Some(config) = self.config.as_mut() {
            if let serde_json::Value::Object(map) = config {
                map.insert(key.to_string(), value);
            }
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value);
            self.config = Some(serde_json::Value::Object(map));
        }
        self
    }

    pub fn get_config_value(&self, key: &str) -> Option<&serde_json::Value> {
        self.config.as_ref().and_then(|config| {
            if let serde_json::Value::Object(map) = config {
                map.get(key)
            } else {
                None
            }
        })
    }

    pub fn set_size(&mut self, size: usize) -> &mut Self {
        // model size in bytes
        self.set_config_value("size", serde_json::Value::Number(size.into()))
    }

    pub fn get_size(&self) -> Option<usize> {
        self.get_config_value("size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
    }

    pub fn set_family(&mut self, family: &str) -> &mut Self {
        self.set_config_value(
            "family",
            serde_json::Value::String(family.to_string()),
        )
    }

    pub fn get_family(&self) -> Option<&str> {
        self.get_config_value("family").and_then(|v| v.as_str())
    }

    pub fn set_description(&mut self, description: &str) -> &mut Self {
        self.set_config_value(
            "description",
            serde_json::Value::String(description.to_string()),
        )
    }

    pub fn get_description(&self) -> Option<&str> {
        self.get_config_value("description")
            .and_then(|v| v.as_str())
    }
}
