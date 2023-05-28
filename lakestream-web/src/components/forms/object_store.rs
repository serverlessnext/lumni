use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use blake3::hash;
use regex::Regex;

use super::helpers::validate_with_pattern;
use crate::stringvault::{ConfigManager, FormInputFieldBuilder, InputData};

#[derive(Debug, Clone)]
pub struct ObjectStore {
    pub name: String,
}

impl ObjectStore {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn id(&self) -> String {
        let hash = hash(self.name.as_bytes());
        hash.to_hex().to_string()
    }

    pub fn tag(&self) -> String {
        "object_store".to_string()
    }

    pub fn default_config(&self) -> HashMap<String, String> {
        Self::default_fields()
            .into_iter()
            .map(|(key, input_data)| (key, input_data.value))
            .collect()
    }

    fn default_fields() -> HashMap<String, InputData> {
        let uri_pattern = Regex::new(r"^s3://").unwrap();
        let aws_key_pattern = Regex::new(r"^.+$").unwrap();
        let aws_secret_pattern = Regex::new(r"^.+$").unwrap();
        let region_pattern = Regex::new(r"^[a-zA-Z0-9\-]*$").unwrap();
        let endpoint_url_pattern = Regex::new(r"^https?://[^/]+/$|^$").unwrap();

        vec![
            FormInputFieldBuilder::new("BUCKET_URI")
                .default("s3://".to_string())
                .validator(Some(Arc::new(validate_with_pattern(
                    uri_pattern,
                    "Invalid URI scheme. Must start with 's3://'.".to_string(),
                ))))
                .build(),
            FormInputFieldBuilder::new("AWS_ACCESS_KEY_ID")
                .default("".to_string())
                .validator(Some(Arc::new(validate_with_pattern(
                    aws_key_pattern,
                    "Invalid AWS access key id.".to_string(),
                ))))
                .build(),
            FormInputFieldBuilder::new("AWS_SECRET_ACCESS_KEY")
                .default("".to_string())
                .secret(true)
                .validator(Some(Arc::new(validate_with_pattern(
                    aws_secret_pattern,
                    "Invalid AWS secret access key.".to_string(),
                ))))
                .build(),
            FormInputFieldBuilder::new("AWS_REGION")
                .default("auto".to_string())
                .validator(Some(Arc::new(validate_with_pattern(
                    region_pattern,
                    "Invalid AWS region.".to_string(),
                ))))
                .build(),
            FormInputFieldBuilder::new("S3_ENDPOINT_URL")
                .default("".to_string())
                .validator(Some(Arc::new(validate_with_pattern(
                    endpoint_url_pattern,
                    "Invalid S3 endpoint URL.".to_string(),
                ))))
                .build(),
        ]
        .into_iter()
        .map(|field| field.to_input_data())
        .collect()
    }

}

#[async_trait(?Send)]
impl ConfigManager for ObjectStore {
    fn get_default_config(&self) -> HashMap<String, String> {
        self.default_config()
    }

    fn default_fields(&self) -> HashMap<String, InputData> {
        ObjectStore::default_fields()
    }

    fn id(&self) -> String {
        Self::id(self)
    }

    fn tag(&self) -> String {
        Self::tag(self)
    }

}
