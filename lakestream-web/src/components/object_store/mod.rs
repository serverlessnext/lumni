use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use blake3::hash;
use regex::Regex;

use crate::stringvault::{ConfigManager, FormInputField, InputData};

mod list;
pub use list::{ObjectStoreList, ObjectStoreListView};

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

    fn default_fields() -> HashMap<String, InputData> {
        let uri_pattern = Regex::new(r"^s3://").unwrap();
        let aws_key_pattern = Regex::new(r"^.+$").unwrap();
        let aws_secret_pattern = Regex::new(r"^.+$").unwrap();
        let region_pattern = Regex::new(r"^[a-zA-Z0-9\-]*$").unwrap();
        let endpoint_url_pattern = Regex::new(r"^https?://[^/]+/$|^$").unwrap();

        vec![
            FormInputField::new(
                "BUCKET_URI",
                "s3://".to_string(),
                Arc::new(validate_with_pattern(
                    uri_pattern,
                    "Invalid URI scheme. Must start with 's3://'.".to_string(),
                )),
            ),
            FormInputField::new(
                "AWS_ACCESS_KEY_ID",
                "".to_string(),
                Arc::new(validate_with_pattern(
                    aws_key_pattern,
                    "Invalid AWS access key id.".to_string(),
                )),
            ),
            FormInputField::new(
                "AWS_SECRET_ACCESS_KEY",
                "".to_string(),
                Arc::new(validate_with_pattern(
                    aws_secret_pattern,
                    "Invalid AWS secret access key.".to_string(),
                )),
            ),
            FormInputField::new(
                "AWS_REGION",
                "auto".to_string(),
                Arc::new(validate_with_pattern(
                    region_pattern,
                    "Invalid AWS region.".to_string(),
                )),
            ),
            FormInputField::new(
                "S3_ENDPOINT_URL",
                "".to_string(),
                Arc::new(validate_with_pattern(
                    endpoint_url_pattern,
                    "Invalid S3 endpoint URL.".to_string(),
                )),
            ),
        ]
        .into_iter()
        .map(|field| field.to_input_data())
        .collect()
    }
}

#[async_trait(?Send)]
impl ConfigManager for ObjectStore {
    fn get_default_config(&self) -> HashMap<String, String> {
        Self::default_fields()
            .into_iter()
            .map(|(key, input_data)| (key, input_data.value))
            .collect()
    }

    fn default_fields(&self) -> HashMap<String, InputData> {
        ObjectStore::default_fields()
    }

    fn id(&self) -> String {
        Self::id(self)
    }

    fn tag(&self) -> String {
        "object_store".to_string()
    }
}

fn validate_with_pattern(
    pattern: Regex,
    error_msg: String,
) -> Box<dyn Fn(&str) -> Result<(), String>> {
    Box::new(move |input: &str| {
        if pattern.is_match(input) {
            Ok(())
        } else {
            Err(error_msg.clone())
        }
    })
}
