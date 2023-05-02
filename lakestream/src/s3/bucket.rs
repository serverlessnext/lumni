use async_trait::async_trait;

pub use super::list::list_buckets;
use super::list::list_files;
use crate::base::config::Config;
use crate::base::interfaces::ObjectStoreTrait;
use crate::s3::config::validate_config;
use crate::{FileObjectFilter, FileObjectVec, LakestreamError};

#[derive(Clone)]
pub struct S3Credentials {
    access_key: String,
    secret_key: String,
}

impl S3Credentials {
    pub fn new(access_key: String, secret_key: String) -> S3Credentials {
        S3Credentials {
            access_key,
            secret_key,
        }
    }

    pub fn access_key(&self) -> &str {
        &self.access_key
    }

    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
}

pub struct S3Bucket {
    name: String,
    config: Config,
}

impl S3Bucket {
    pub fn new(
        name: &str,
        mut config: Config,
    ) -> Result<S3Bucket, LakestreamError> {
        validate_config(&mut config)?;

        Ok(S3Bucket {
            name: name.to_string(),
            config,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn bucket_path(&self) -> String {
        let region = self.config.settings.get("AWS_REGION").unwrap();
        let endpoint_url = self
            .config
            .settings
            .get("S3_ENDPOINT_URL")
            .map(String::as_str);
        let name = Some(self.name().to_string());

        configure_bucket_url(region, endpoint_url, name.as_deref())
    }
}

#[async_trait(?Send)]
impl ObjectStoreTrait for S3Bucket {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &Config {
        &self.config
    }
    async fn list_files(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
        file_objects: &mut FileObjectVec,
    ) -> Result<(), LakestreamError> {
        list_files(self, prefix, recursive, max_keys, filter, file_objects)
            .await
    }
}

pub fn configure_bucket_url(
    region: &str,
    endpoint_url: Option<&str>,
    bucket_name: Option<&str>,
) -> String {
    match endpoint_url {
        Some(url) => match bucket_name {
            Some(name) => format!("{}/{}", url.trim_end_matches('/'), name),
            None => url.to_owned(),
        },
        None => match bucket_name {
            Some(name) => {
                format!("https://{}.s3.{}.amazonaws.com", name, region)
            }
            None => format!("https://s3.{}.amazonaws.com", region),
        },
    }
}
