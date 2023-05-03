use async_trait::async_trait;
use log::error;

pub use super::bucket::S3Bucket;
pub use super::config::validate_config;
pub use super::list::list_buckets;
use crate::{Config, LakestreamError, ObjectStore, ObjectStoreBackend};

pub struct S3Backend;

#[async_trait]
impl ObjectStoreBackend for S3Backend {
    fn new(_config: Config) -> Result<Self, LakestreamError> {
        Ok(Self)
    }

    async fn list_buckets(
        config: Config,
    ) -> Result<Vec<ObjectStore>, LakestreamError> {
        let config_map = config.settings.clone();
        let mut config_instance = Config {
            settings: config_map,
        };
        if let Err(e) = validate_config(&mut config_instance) {
            // Handle the error, e.g., log the error and/or return early with an appropriate error value
            error!("Error validating the config: {}", e);
            return Err(LakestreamError::ConfigError(
                "Invalid configuration".to_string(),
            ));
        }
        list_buckets(&config_instance).await
    }
}
