use std::collections::HashMap;
use std::sync::Arc;

use regex::Regex;
use uuid::Uuid;

use crate::components::form_input::{
    validate_with_pattern, FormFieldBuilder, FormElement, InputFieldData, FieldType,
};

#[derive(Debug, Clone)]
pub struct ObjectStoreForm {
    name: String,
    id: String,
}

impl ObjectStoreForm {
    pub fn new(name: String) -> Self {
        Self {
            name,
            id: Uuid::new_v4().to_string(),
        }
    }

    pub fn new_with_id(name: String, id: String) -> Self {
        Self { name, id }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn default_fields<S: Into<String>>(name: S) -> HashMap<String, InputFieldData> {
        let uri_pattern = Regex::new(r"^s3://").unwrap();
        let aws_key_pattern = Regex::new(r"^.+$").unwrap();
        let aws_secret_pattern = Regex::new(r"^.+$").unwrap();
        let region_pattern = Regex::new(r"^[a-zA-Z0-9\-]*$").unwrap();
        let endpoint_url_pattern = Regex::new(r"^https?://[^/]+/$|^$").unwrap();

        let fields = vec![
            FormFieldBuilder::new("__NAME__")
                .default(name)
                .label("Name")
                .validator(None)
                .build(),
            FormFieldBuilder::new("BUCKET_URI")
                .default("s3://")
                .label("Bucket URI")
                .validator(Some(Arc::new(validate_with_pattern(
                    uri_pattern,
                    "Invalid URI scheme. Must start with 's3://'.".to_string(),
                ))))
                .build(),
            FormFieldBuilder::new("AWS_ACCESS_KEY_ID")
                .default("")
                .label("AWS Access Key ID")
                .validator(Some(Arc::new(validate_with_pattern(
                    aws_key_pattern,
                    "Invalid AWS access key id.".to_string(),
                ))))
                .build(),
            FormFieldBuilder::new("AWS_SECRET_ACCESS_KEY")
                .default("")
                .label("AWS Secret Access Key")
                .field_type(FieldType::Secret)
                .validator(Some(Arc::new(validate_with_pattern(
                    aws_secret_pattern,
                    "Invalid AWS secret access key.".to_string(),
                ))))
                .build(),
            FormFieldBuilder::new("AWS_REGION")
                .default("auto")
                .label("AWS Region")
                .validator(Some(Arc::new(validate_with_pattern(
                    region_pattern,
                    "Invalid AWS region.".to_string(),
                ))))
                .build(),
            FormFieldBuilder::new("S3_ENDPOINT_URL")
                .default("")
                .label("S3 Endpoint URL")
                .validator(Some(Arc::new(validate_with_pattern(
                    endpoint_url_pattern,
                    "Invalid S3 endpoint URL.".to_string(),
                ))))
                .build(),
        ]
        .into_iter()
        .map(|field| {
            match field {
                FormElement::InputField(field_data) => {
                    (field_data.name.clone(), field_data)
                },
            }
        })
        .collect();

        fields
    }
}
