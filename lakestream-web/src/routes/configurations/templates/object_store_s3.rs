use std::sync::Arc;

use regex::Regex;

use crate::components::form_input::{
    build_all, validate_with_pattern, FieldBuilder, FieldType, FormElement,
    TextBoxBuilder,
};

pub fn form_elements<S: Into<String>>(name: S) -> Vec<FormElement> {
    let uri_pattern = Regex::new(r"^s3://").unwrap();
    let aws_key_pattern = Regex::new(r"^.+$").unwrap();
    let aws_secret_pattern = Regex::new(r"^.+$").unwrap();
    let region_pattern = Regex::new(r"^[a-zA-Z0-9\-]*$").unwrap();
    let endpoint_url_pattern = Regex::new(r"^https?://[^/]+/$|^$").unwrap();

    let builders: Vec<TextBoxBuilder> = vec![
        TextBoxBuilder::from(FieldBuilder::new("__NAME__").label("Name"))
            .default(name)
            .validator(None),
        TextBoxBuilder::from(
            FieldBuilder::new("BUCKET_URI").label("Bucket URI"),
        )
        .default("s3://")
        .validator(Some(Arc::new(validate_with_pattern(
            uri_pattern,
            "Invalid URI scheme. Must start with 's3://'.".to_string(),
        )))),
        TextBoxBuilder::from(
            FieldBuilder::new("AWS_ACCESS_KEY_ID").label("AWS Access Key ID"),
        )
        .default("")
        .validator(Some(Arc::new(validate_with_pattern(
            aws_key_pattern,
            "Invalid AWS access key id.".to_string(),
        )))),
        TextBoxBuilder::from(
            FieldBuilder::new("AWS_SECRET_ACCESS_KEY")
                .label("AWS Secret Access Key"),
        )
        .default("")
        .field_type(FieldType::Secret)
        .validator(Some(Arc::new(validate_with_pattern(
            aws_secret_pattern,
            "Invalid AWS secret access key.".to_string(),
        )))),
        TextBoxBuilder::from(
            FieldBuilder::new("AWS_REGION").label("AWS Region"),
        )
        .default("auto")
        .validator(Some(Arc::new(validate_with_pattern(
            region_pattern,
            "Invalid AWS region.".to_string(),
        )))),
        TextBoxBuilder::from(
            FieldBuilder::new("S3_ENDPOINT_URL").label("S3 Endpoint URL"),
        )
        .default("")
        .validator(Some(Arc::new(validate_with_pattern(
            endpoint_url_pattern,
            "Invalid S3 endpoint URL.".to_string(),
        )))),
    ];

    let elements: Vec<FormElement> = build_all(builders);
    elements
}
