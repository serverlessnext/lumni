use std::fs::{self, ReadDir};
use std::io;
use std::path::Path;

use async_trait::async_trait;

use super::get::get_object;
use super::list::list_files;
use crate::base::config::EnvironmentConfig;
use crate::{
    FileObjectFilter, FileObjectVec, LakestreamError, ObjectStoreTrait,
};

pub struct LocalFileSystem;

pub trait FileSystem {
    fn read_dir(&self, path: &Path) -> io::Result<ReadDir>;
}

impl FileSystem for LocalFileSystem {
    fn read_dir(&self, path: &Path) -> io::Result<ReadDir> {
        fs::read_dir(path)
    }
}

#[derive(Clone)]
pub struct LocalFsBucket {
    name: String,
    #[allow(dead_code)]
    config: EnvironmentConfig,
}

impl LocalFsBucket {
    pub fn new(
        name: &str,
        config: EnvironmentConfig,
    ) -> Result<LocalFsBucket, &'static str> {
        Ok(LocalFsBucket {
            name: name.to_string(),
            config,
        })
    }
}

#[async_trait(?Send)]
impl ObjectStoreTrait for LocalFsBucket {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &EnvironmentConfig {
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
        let path = match prefix {
            Some(prefix) => Path::new(&self.name).join(prefix),
            None => Path::new(&self.name).to_path_buf(),
        };

        // to be considered a Bucket, path must be a directory
        if !path.is_dir() {
            return Err(LakestreamError::NoBucketInUri(
                path.to_string_lossy().to_string(),
            ));
        }
        list_files(&path, max_keys, recursive, filter, file_objects).await;
        Ok(())
    }

    async fn get_object(
        &self,
        key: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), LakestreamError> {
        let path = Path::new(&self.name);
        get_object(path, key, data).await
    }
}
