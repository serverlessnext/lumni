use std::collections::HashMap;
use super::config::update_config;
use async_trait::async_trait;
use futures::FutureExt;

pub use super::list::list_buckets;
use super::list::list_files;
use crate::base::interfaces::ObjectStoreTrait;
use crate::{FileObject, FileObjectFilter};
use crate::LakestreamError;

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
    config: HashMap<String, String>,
}

impl S3Bucket {
    pub fn new(
        name: &str,
        config: HashMap<String, String>,
    ) -> Result<S3Bucket, LakestreamError> {
        let updated_config = update_config(&config)?;

        Ok(S3Bucket {
            name: name.to_string(),
            config: updated_config,
        })
    }
}

#[async_trait(?Send)]
impl ObjectStoreTrait for S3Bucket {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &HashMap<String, String> {
        &self.config
    }

    async fn list_files(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
    ) -> Result<Vec<FileObject>, LakestreamError> {
        let result: Result<Vec<FileObject>, LakestreamError> = async move {
            list_files(self, prefix, recursive, max_keys, filter).await
        }
        .boxed_local() // Use boxed_local() here
        .await;

        match result {
            Ok(files) => Ok(files),
            Err(e) => Err(e.into()),
        }
    }
}
