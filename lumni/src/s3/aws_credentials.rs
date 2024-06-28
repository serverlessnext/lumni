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
