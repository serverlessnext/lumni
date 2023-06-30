use std::sync::Arc;

use regex::Regex;

use crate::builders::{
    build_all, FieldBuilder, FieldBuilderTrait, TextBoxBuilder,
};
use crate::components::form_input::{
    validate_with_pattern, FieldType, FormElement,
};

pub fn form_elements<S: Into<String>>(
    name: S,
) -> Vec<Box<dyn FieldBuilderTrait>> {
    let uri_pattern = Regex::new(r"^s3://").unwrap();
    let aws_key_pattern = Regex::new(r"^.+$").unwrap();
    let aws_secret_pattern = Regex::new(r"^.+$").unwrap();
    let region_pattern = Regex::new(r"^[a-zA-Z0-9\-]*$").unwrap();
    let endpoint_url_pattern = Regex::new(r"^https?://[^/]+/$|^$").unwrap();

    let builders: Vec<Box<dyn FieldBuilderTrait>> = vec![
        Box::new(
            TextBoxBuilder::from(
                FieldBuilder::new("__NAME__").with_label("Name"),
            )
            .with_initial_value(name)
            .validator(None),
        ),
        Box::new(
            TextBoxBuilder::from(
                FieldBuilder::new("BUCKET_URI").with_label("Bucket URI"),
            )
            .with_initial_value("s3://")
            .validator(Some(Arc::new(validate_with_pattern(
                uri_pattern,
                "Invalid URI scheme. Must start with 's3://'.".to_string(),
            )))),
        ),
        Box::new(
            TextBoxBuilder::from(
                FieldBuilder::new("AWS_ACCESS_KEY_ID")
                    .with_label("AWS Access Key ID"),
            )
            .validator(Some(Arc::new(validate_with_pattern(
                aws_key_pattern,
                "Invalid AWS access key id.".to_string(),
            )))),
        ),
        Box::new(
            TextBoxBuilder::from(
                FieldBuilder::new("AWS_SECRET_ACCESS_KEY")
                    .with_label("AWS Secret Access Key"),
            )
            .field_type(FieldType::Secret)
            .validator(Some(Arc::new(validate_with_pattern(
                aws_secret_pattern,
                "Invalid AWS secret access key.".to_string(),
            )))),
        ),
        Box::new(
            TextBoxBuilder::from(
                FieldBuilder::new("AWS_REGION").with_label("AWS Region"),
            )
            .with_initial_value("auto")
            .validator(Some(Arc::new(validate_with_pattern(
                region_pattern,
                "Invalid AWS region.".to_string(),
            )))),
        ),
        Box::new(
            TextBoxBuilder::from(
                FieldBuilder::new("S3_ENDPOINT_URL")
                    .with_label("S3 Endpoint URL"),
            )
            .validator(Some(Arc::new(validate_with_pattern(
                endpoint_url_pattern,
                "Invalid S3 endpoint URL.".to_string(),
            )))),
        ),
    ];

    builders
}
