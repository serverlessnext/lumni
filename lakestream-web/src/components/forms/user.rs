use std::collections::HashMap;
use std::sync::Arc;

use regex::Regex;
use uuid::Uuid;

use super::helpers::validate_with_pattern;
use crate::stringvault::{ConfigManager, FormInputFieldBuilder, InputData};

#[derive(Debug, Clone)]
pub struct UserForm {
    pub name: String,
    pub id: String,
}

impl UserForm {
    pub fn new(name: String) -> Self {
        Self {
            name,
            id: Uuid::new_v4().to_string(),
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    fn default_fields(&self) -> HashMap<String, InputData> {
        let password_pattern = Regex::new(r"^.{8,}$").unwrap();

        vec![FormInputFieldBuilder::new("PASSWORD")
            .default("".to_string())
            .password(true)
            .validator(Some(Arc::new(validate_with_pattern(
                password_pattern,
                "Invalid password. Must be at least 8 characters.".to_string(),
            ))))
            .build()]
        .into_iter()
        .map(|field| field.to_input_data())
        .collect()
    }
}

impl ConfigManager for UserForm {
    fn default_fields(&self) -> HashMap<String, InputData> {
        self.default_fields()
    }

    fn id(&self) -> String {
        self.id()
    }
}
