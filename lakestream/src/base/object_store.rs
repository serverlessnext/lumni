use async_trait::async_trait;

use crate::localfs::bucket::LocalFs;
use crate::s3::bucket::S3Bucket;
use crate::{Config, FileObject, FileObjectFilter, LakestreamError};

pub enum ObjectStore {
    S3Bucket(S3Bucket),
    LocalFs(LocalFs),
}

impl ObjectStore {
    pub fn new(name: &str, config: Config) -> Result<ObjectStore, String> {
        if name.starts_with("s3://") {
            let name = name.trim_start_matches("s3://");
            let bucket =
                S3Bucket::new(name, config).map_err(|err| err.to_string())?;
            Ok(ObjectStore::S3Bucket(bucket))
        } else if name.starts_with("localfs://") {
            let name = name.trim_start_matches("localfs://");
            let local_fs =
                LocalFs::new(name, config).map_err(|err| err.to_string())?;
            Ok(ObjectStore::LocalFs(local_fs))
        } else {
            Err("Unsupported object store.".to_string())
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.name(),
            ObjectStore::LocalFs(local_fs) => local_fs.name(),
        }
    }

    pub fn config(&self) -> &Config {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.config(),
            ObjectStore::LocalFs(local_fs) => local_fs.config(),
        }
    }

    pub async fn list_files(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
    ) -> Result<Vec<FileObject>, LakestreamError> {
        match self {
            ObjectStore::S3Bucket(bucket) => {
                match bucket
                    .list_files(prefix, recursive, max_keys, filter)
                    .await
                {
                    Ok(files) => Ok(files),
                    Err(e) => Err(e),
                }
            }
            ObjectStore::LocalFs(local_fs) => {
                match local_fs
                    .list_files(prefix, recursive, max_keys, filter)
                    .await
                {
                    Ok(files) => Ok(files),
                    Err(e) => Err(e),
                }
            }
        }
    }
}

#[async_trait(?Send)]
pub trait ObjectStoreTrait {
    fn name(&self) -> &str;
    fn config(&self) -> &Config;
    async fn list_files(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
    ) -> Result<Vec<FileObject>, LakestreamError>;
}
