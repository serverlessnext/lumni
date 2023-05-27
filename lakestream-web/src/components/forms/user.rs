
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use blake3::hash;
use regex::Regex;

use crate::stringvault::{ConfigManager, FormInputFieldBuilder, FormInputField, InputData, InputElementOpts};
use super::helpers::validate_with_pattern;


#[derive(Debug, Clone)]
pub struct UserForm {
    pub name: String,
}

impl UserForm {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn id(&self) -> String {
        let hash = hash(self.name.as_bytes());
        hash.to_hex().to_string()
    }

    fn default_fields() -> HashMap<String, InputData> {
        let username_pattern = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
        let password_pattern = Regex::new(r"^.{8,}$").unwrap();

        vec![
            FormInputFieldBuilder::new("USERNAME")
                .default("".to_string())
                .enabled(false)
                .validator(Some(Arc::new(validate_with_pattern(
                    username_pattern,
                    "Invalid username. Must contain only alphanumeric characters and underscores.".to_string(),
                ))))
                .build(),
            FormInputFieldBuilder::new("PASSWORD")
                .default("".to_string())
                .secret(true)
                .validator(Some(Arc::new(validate_with_pattern(
                    password_pattern,
                    "Invalid password. Must be at least 8 characters.".to_string(),
                ))))
                .build(),
        ]
        .into_iter()
        .map(|field| field.to_input_data())
        .collect()
    }
}

#[async_trait(?Send)]
impl ConfigManager for UserForm {
    fn get_default_config(&self) -> HashMap<String, String> {
        Self::default_fields()
            .into_iter()
            .map(|(key, input_data)| (key, input_data.value))
            .collect()
    }

    fn default_fields(&self) -> HashMap<String, InputData> {
        UserForm::default_fields()
    }

    fn id(&self) -> String {
        Self::id(self)
    }

    fn tag(&self) -> String {
        "admin_form".to_string()
    }
}

