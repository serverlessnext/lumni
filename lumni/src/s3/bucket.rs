use std::collections::HashMap;

use async_trait::async_trait;

use super::get::get_object;
use super::head::head_object;
use super::list::list_files;
use crate::base::config::EnvironmentConfig;
use crate::handlers::object_store::ObjectStoreTrait;
use crate::s3::config::validate_config;
use crate::table::FileObjectTable;
use crate::{FileObjectFilter, LakestreamError};

#[derive(Debug, Clone)]
pub struct S3Bucket {
    name: String,
    config: EnvironmentConfig,
}

impl S3Bucket {
    pub fn new(
        name: &str,
        mut config: EnvironmentConfig,
    ) -> Result<S3Bucket, LakestreamError> {
        validate_config(&mut config)?;

        Ok(S3Bucket {
            name: name.to_string(),
            config,
        })
    }

    pub fn config(&self) -> &EnvironmentConfig {
        &self.config
    }

    pub fn bucket_path(&self) -> String {
        let region = self.config.get("AWS_REGION").unwrap();
        let endpoint_url =
            self.config.get("S3_ENDPOINT_URL").map(String::as_str);
        let name = Some(self.name().to_string());

        configure_bucket_url(region, endpoint_url, name.as_deref())
    }
}

#[async_trait(?Send)]
impl ObjectStoreTrait for S3Bucket {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &EnvironmentConfig {
        &self.config
    }
    async fn list_files(
        &self,
        prefix: Option<&str>,
        selected_columns: &Option<Vec<&str>>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
        table: &mut FileObjectTable,
    ) -> Result<(), LakestreamError> {
        if let Some(prefix) = prefix {
            // prefix should not exist as a file object
            let (status_code, _response_headers) =
                self.head_object(prefix.trim_end_matches("/")).await?;
            if status_code != 404 {
                return Err(LakestreamError::NoBucketInUri(prefix.to_string()));
            }
        }
        list_files(
            self,
            prefix,
            selected_columns,
            recursive,
            max_keys,
            filter,
            table,
        )
        .await
    }

    async fn get_object(
        &self,
        key: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), LakestreamError> {
        get_object(self, key, data).await
    }

    async fn head_object(
        &self,
        key: &str,
    ) -> Result<(u16, HashMap<String, String>), LakestreamError> {
        head_object(self, key).await
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
