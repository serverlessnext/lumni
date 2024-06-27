use std::env;

use crate::LakestreamError;

pub const AWS_DEFAULT_REGION: &str = "us-east-1";

#[derive(Clone)]
pub struct AWSCredentials {
    access_key: String,
    secret_key: String,
    region: String,
    session_token: Option<String>,
}

impl AWSCredentials {
    pub fn new(
        access_key: String,
        secret_key: String,
        region: String,
        session_token: Option<String>,
    ) -> AWSCredentials {
        AWSCredentials {
            access_key,
            secret_key,
            region,
            session_token,
        }
    }

    pub fn from_env() -> Result<AWSCredentials, LakestreamError> {
        let access_key = env::var("AWS_ACCESS_KEY_ID").map_err(|_| {
            LakestreamError::ConfigError(
                "AWS_ACCESS_KEY_ID not found in the config and environment"
                    .to_string(),
            )
        })?;
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| {
            LakestreamError::ConfigError(
                "AWS_SECRET_ACCESS_KEY not found in the config and environment"
                    .to_string(),
            )
        })?;
        let region = env::var("AWS_REGION").unwrap_or_else(|_| {
            env::var("AWS_DEFAULT_REGION")
                .unwrap_or_else(|_| AWS_DEFAULT_REGION.to_owned())
        });
        let session_token = env::var("AWS_SESSION_TOKEN").ok();

        Ok(AWSCredentials {
            access_key,
            secret_key,
            region,
            session_token,
        })
    }

    pub fn access_key(&self) -> &str {
        &self.access_key
    }

    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }

    pub fn session_token(&self) -> Option<&str> {
        self.session_token.as_deref()
    }

    pub fn region(&self) -> &str {
        &self.region
    }
}

// OLD
//pub fn validate_config(
//    config: &mut EnvironmentConfig,
//) -> Result<(), LakestreamError> {
//    // Set AWS_REGION
//    if !config.contains_key("AWS_REGION") {
//        if let Some(region) = config.get("region").cloned() {
//            config.insert("AWS_REGION".to_string(), region);
//        } else {
//            let region = env::var("AWS_REGION").unwrap_or_else(|_| {
//                env::var("AWS_DEFAULT_REGION")
//                    .unwrap_or_else(|_| AWS_DEFAULT_REGION.to_owned())
//            });
//            config.insert("AWS_REGION".to_string(), region);
//        }
//    }
//
//    // Set AWS_ACCESS_KEY_ID
//    if !config.contains_key("AWS_ACCESS_KEY_ID") {
//        if let Ok(aws_access_key_id) = env::var("AWS_ACCESS_KEY_ID") {
//            config.insert("AWS_ACCESS_KEY_ID".to_string(), aws_access_key_id);
//        } else {
//            return Err(LakestreamError::ConfigError(
//                "AWS_ACCESS_KEY_ID not found in the config and environment"
//                    .to_string(),
//            ));
//        }
//    }
//
//    // Set AWS_SECRET_ACCESS_KEY
//    if !config.contains_key("AWS_SECRET_ACCESS_KEY") {
//        if let Ok(aws_secret_access_key) = env::var("AWS_SECRET_ACCESS_KEY") {
//            config.insert(
//                "AWS_SECRET_ACCESS_KEY".to_string(),
//                aws_secret_access_key,
//            );
//        } else {
//            return Err(LakestreamError::ConfigError(
//                "AWS_SECRET_ACCESS_KEY not found in the config and environment"
//                    .to_string(),
//            ));
//        }
//    }
//
//    // Set AWS_SESSION_TOKEN (optional)
//    if !config.contains_key("AWS_SESSION_TOKEN") {
//        if let Ok(aws_session_token) = env::var("AWS_SESSION_TOKEN") {
//            config.insert("AWS_SESSION_TOKEN".to_string(), aws_session_token);
//        }
//    }
//
//    // Set AWS Endpoint (optional)
//    if !config.contains_key("S3_ENDPOINT_URL") {
//        if let Ok(s3_endpoint_url) = env::var("S3_ENDPOINT_URL") {
//            config.insert("S3_ENDPOINT_URL".to_string(), s3_endpoint_url);
//        }
//    }
//
//    Ok(())
//}
//
