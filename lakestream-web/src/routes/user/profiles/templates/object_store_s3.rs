use std::sync::Arc;

use regex::Regex;

use crate::builders::ElementBuilder;
use crate::components::form_input::{validate_with_pattern, FieldType};

pub fn form_elements<S: Into<String>>(name: S) -> Vec<ElementBuilder> {
    let uri_pattern = Regex::new(r"^s3://").unwrap();
    let aws_key_pattern = Regex::new(r"^.+$").unwrap();
    let aws_secret_pattern = Regex::new(r"^.+$").unwrap();
    let region_pattern = Regex::new(r"^[a-zA-Z0-9\-]*$").unwrap();
    let endpoint_url_pattern = Regex::new(r"^https?://[^/]+/$|^$").unwrap();

    let builders: Vec<ElementBuilder> = vec![
        ElementBuilder::new("__NAME__", FieldType::Text)
            .with_label("Name")
            .with_initial_value(name)
            .validator(None),
        ElementBuilder::new("BUCKET_URI", FieldType::Text)
            .with_label("Bucket URI")
            .with_initial_value("s3://")
            .validator(Some(Arc::new(validate_with_pattern(
                uri_pattern,
                "Invalid URI scheme. Must start with 's3://'.".to_string(),
            )))),
        ElementBuilder::new("AWS_ACCESS_KEY_ID", FieldType::Text)
            .with_label("AWS Access Key ID")
            .validator(Some(Arc::new(validate_with_pattern(
                aws_key_pattern,
                "Invalid AWS access key id.".to_string(),
            )))),
        ElementBuilder::new("AWS_SECRET_ACCESS_KEY", FieldType::Secret)
            .with_label("AWS Secret Access Key")
            .validator(Some(Arc::new(validate_with_pattern(
                aws_secret_pattern,
                "Invalid AWS secret access key.".to_string(),
            )))),
        ElementBuilder::new("AWS_REGION", FieldType::Text)
            .with_label("AWS Region")
            .with_initial_value("auto")
            .validator(Some(Arc::new(validate_with_pattern(
                region_pattern,
                "Invalid AWS region.".to_string(),
            )))),
        ElementBuilder::new("S3_ENDPOINT_URL", FieldType::Text)
            .with_label("S3 Endpoint URL")
            .validator(Some(Arc::new(validate_with_pattern(
                endpoint_url_pattern,
                "Invalid S3 endpoint URL.".to_string(),
            )))),
    ];

    builders
}
