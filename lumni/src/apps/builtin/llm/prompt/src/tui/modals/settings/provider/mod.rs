mod creator;

use std::collections::HashMap;

pub use creator::{ProviderCreationStep, ProviderCreator};
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub model_identifier: Option<String>,
    pub additional_settings: HashMap<String, ProviderConfigOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigOptions {
    pub name: String,
    pub display_name: String,
    pub value: String,
    pub is_secure: bool,
    pub placeholder: String,
}
