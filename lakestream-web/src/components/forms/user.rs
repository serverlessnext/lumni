use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use blake3::hash;
use regex::Regex;

use super::helpers::validate_with_pattern;
use crate::stringvault::{
    ConfigManager, FormInputFieldBuilder, InputData,
};

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

    fn default_fields(username: &str) -> HashMap<String, InputData> {
        let password_pattern = Regex::new(r"^.{8,}$").unwrap();

        vec![
            FormInputFieldBuilder::new("PASSWORD")
                .default("".to_string())
                .enabled(false)
                .secret(true)
                .validator(Some(Arc::new(validate_with_pattern(
                    password_pattern,
                    "Invalid password. Must be at least 8 characters."
                        .to_string(),
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
        Self::default_fields(&self.name)
            .into_iter()
            .map(|(key, input_data)| (key, input_data.value))
            .collect()
    }

    fn default_fields(&self) -> HashMap<String, InputData> {
        // for UserForm, name is the username
        UserForm::default_fields(&self.name)
    }

    fn id(&self) -> String {
        Self::id(self)
    }

    fn tag(&self) -> String {
        "user_form".to_string()
    }
}
