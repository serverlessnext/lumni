use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use log::{error, info, warn};

use crate::{ApplicationError, LumniError};
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

    pub fn from_env() -> Result<AWSCredentials, LumniError> {
        if let Ok(credentials) = Self::from_env_vars() {
            info!("Loaded AWS credentials from environment variables");
            return Ok(credentials);
        }

        if let Some(profile) = env::var("AWS_PROFILE")
            .or_else(|_| env::var("AWS_DEFAULT_PROFILE"))
            .ok()
        {
            info!("Loading AWS credentials from profile '{}'", profile);
            return Self::from_profile(&profile);
        }

        error!("No valid AWS credentials found");
        Err(LumniError::Application(
            ApplicationError::InvalidCredentials(
                "No valid AWS credentials found".to_string(),
            ),
            None,
        ))
    }

    fn from_env_vars() -> Result<AWSCredentials, LumniError> {
        let access_key = env::var("AWS_ACCESS_KEY_ID").map_err(|_| {
            error!("AWS_ACCESS_KEY_ID not found in environment");
            LumniError::Application(
                ApplicationError::InvalidCredentials(
                    "AWS_ACCESS_KEY_ID not found in environment".to_string(),
                ),
                None,
            )
        })?;
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| {
            error!("AWS_SECRET_ACCESS_KEY not found in environment");
            LumniError::Application(
                ApplicationError::InvalidCredentials(
                    "AWS_SECRET_ACCESS_KEY not found in environment"
                        .to_string(),
                ),
                None,
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

    pub fn from_profile(
        profile_name: &str,
    ) -> Result<AWSCredentials, LumniError> {
        let credentials_path = Self::get_credentials_path();
        let config_path = Self::get_config_path();

        info!(
            "Loading AWS credentials for profile '{}' from {:?}",
            profile_name, credentials_path
        );
        let credentials =
            Self::parse_credentials_file(&credentials_path, profile_name)?;

        info!(
            "Loading AWS config for profile '{}' from {:?}",
            profile_name, config_path
        );
        let config = Self::parse_config_file(&config_path, profile_name)?;

        Ok(AWSCredentials {
            access_key: credentials.access_key,
            secret_key: credentials.secret_key,
            region: config.region,
            session_token: credentials.session_token,
        })
    }

    fn get_credentials_path() -> PathBuf {
        env::var("AWS_SHARED_CREDENTIALS_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("Unable to determine home directory")
                    .join(".aws")
                    .join("credentials")
            })
    }

    fn get_config_path() -> PathBuf {
        env::var("AWS_CONFIG_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("Unable to determine home directory")
                    .join(".aws")
                    .join("config")
            })
    }

    fn parse_credentials_file(
        path: &Path,
        profile_name: &str,
    ) -> Result<AWSCredentials, LumniError> {
        let file = File::open(path).map_err(|e| {
            error!("Unable to open AWS credentials file at {:?}: {}", path, e);
            LumniError::Application(
                ApplicationError::InvalidCredentials(format!(
                    "Unable to open AWS credentials file at {:?}: {}",
                    path, e
                )),
                None,
            )
        })?;

        let reader = BufReader::new(file);
        let mut in_profile = false;
        let mut access_key = None;
        let mut secret_key = None;
        let mut session_token = None;

        for (line_number, line_result) in reader.lines().enumerate() {
            let line = line_result.map_err(|e| {
                error!(
                    "Error reading line {} in AWS credentials file: {}",
                    line_number + 1,
                    e
                );
                LumniError::Application(
                    ApplicationError::InvalidCredentials(format!(
                        "Error reading AWS credentials file: {}",
                        e
                    )),
                    None,
                )
            })?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') {
                // Extract profile name, ignoring any comments
                let profile = line
                    .trim_start_matches('[')
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches(']');
                in_profile = profile == profile_name;
                if in_profile {
                    info!(
                        "Found profile '{}' in credentials file",
                        profile_name
                    );
                }
                continue;
            }

            if in_profile {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].split('#').next().unwrap_or("").trim();
                    match key {
                        "aws_access_key_id" => {
                            access_key = Some(value.to_string())
                        }
                        "aws_secret_access_key" => {
                            secret_key = Some(value.to_string())
                        }
                        "aws_session_token" => {
                            session_token = Some(value.to_string())
                        }
                        _ => warn!(
                            "Unknown key '{}' in profile '{}' of credentials \
                             file",
                            key, profile_name
                        ),
                    }
                }
            }
        }

        match (access_key.clone(), secret_key.clone()) {
            (Some(access_key), Some(secret_key)) => {
                info!(
                    "Successfully loaded credentials for profile '{}'",
                    profile_name
                );
                Ok(AWSCredentials {
                    access_key,
                    secret_key,
                    region: AWS_DEFAULT_REGION.to_string(), // Will be overwritten by config file
                    session_token,
                })
            }
            _ => {
                error!(
                    "Invalid or missing credentials for profile '{}'. Access \
                     key present: {}, Secret key present: {}",
                    profile_name,
                    access_key.is_some(),
                    secret_key.is_some()
                );
                Err(LumniError::Application(
                    ApplicationError::InvalidCredentials(format!(
                        "Invalid or missing credentials for profile '{}'. \
                         Access key present: {}, Secret key present: {}",
                        profile_name,
                        access_key.is_some(),
                        secret_key.is_some()
                    )),
                    None,
                ))
            }
        }
    }

    fn parse_config_file(
        path: &Path,
        profile_name: &str,
    ) -> Result<AWSCredentials, LumniError> {
        let file = File::open(path).map_err(|e| {
            error!("Unable to open AWS config file at {:?}: {}", path, e);
            LumniError::Application(
                ApplicationError::InvalidCredentials(format!(
                    "Unable to open AWS config file at {:?}: {}",
                    path, e
                )),
                None,
            )
        })?;

        let reader = BufReader::new(file);
        let mut in_profile = false;
        let mut region = None;

        for (line_number, line_result) in reader.lines().enumerate() {
            let line = line_result.map_err(|e| {
                error!(
                    "Error reading line {} in AWS config file: {}",
                    line_number + 1,
                    e
                );
                LumniError::Application(
                    ApplicationError::InvalidCredentials(format!(
                        "Error reading AWS config file: {}",
                        e
                    )),
                    None,
                )
            })?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') {
                // Extract profile name, ignoring any comments
                let profile = line
                    .trim_start_matches('[')
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches(']');
                in_profile = profile == format!("profile {}", profile_name);
                if in_profile {
                    info!("Found profile '{}' in config file", profile_name);
                }
                continue;
            }

            if in_profile {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 && parts[0].trim() == "region" {
                    let value = parts[1].split('#').next().unwrap_or("").trim();
                    region = Some(value.to_string());
                    info!(
                        "Found region '{}' for profile '{}'",
                        value, profile_name
                    );
                    break;
                }
            }
        }

        if let Some(region) = region {
            Ok(AWSCredentials {
                access_key: String::new(),
                secret_key: String::new(),
                region,
                session_token: None,
            })
        } else {
            warn!(
                "No region found for profile '{}' in config file, using \
                 default region",
                profile_name
            );
            Ok(AWSCredentials {
                access_key: String::new(),
                secret_key: String::new(),
                region: AWS_DEFAULT_REGION.to_string(),
                session_token: None,
            })
        }
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
