use std::env;

use crate::{EnvironmentConfig, LakestreamError};

const AWS_DEFAULT_REGION: &str = "us-east-1";

pub fn validate_config(config: &mut EnvironmentConfig) -> Result<(), LakestreamError> {
    // Set AWS_REGION
    if !config.contains_key("AWS_REGION") {
        if let Some(region) = config.get("region").cloned() {
            config.insert("AWS_REGION".to_string(), region);
        } else {
            let region = env::var("AWS_REGION").unwrap_or_else(|_| {
                env::var("AWS_DEFAULT_REGION")
                    .unwrap_or_else(|_| AWS_DEFAULT_REGION.to_owned())
            });
            config.insert("AWS_REGION".to_string(), region);
        }
    }

    // Set AWS Endpoint
    if !config.contains_key("S3_ENDPOINT_URL") {
        if let Ok(s3_endpoint_url) = env::var("S3_ENDPOINT_URL") {
            config.insert("S3_ENDPOINT_URL".to_string(), s3_endpoint_url);
        }
    }

    // Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY
    if !config.contains_key("AWS_ACCESS_KEY_ID") {
        if let Ok(aws_access_key_id) = env::var("AWS_ACCESS_KEY_ID") {
            config.insert("AWS_ACCESS_KEY_ID".to_string(), aws_access_key_id);
        } else {
            return Err(LakestreamError::ConfigError(
                "AWS_ACCESS_KEY_ID not found in the config and environment"
                    .to_string(),
            ));
        }
    }

    if !config.contains_key("AWS_SECRET_ACCESS_KEY") {
        if let Ok(aws_secret_access_key) = env::var("AWS_SECRET_ACCESS_KEY") {
            config.insert(
                "AWS_SECRET_ACCESS_KEY".to_string(),
                aws_secret_access_key,
            );
        } else {
            return Err(LakestreamError::ConfigError(
                "AWS_SECRET_ACCESS_KEY not found in the config and environment"
                    .to_string(),
            ));
        }
    }

    // Any other custom logic related to the S3 object store
    Ok(())
}
