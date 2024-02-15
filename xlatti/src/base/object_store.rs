use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Future;

use crate::base::callback_wrapper::CallbackItem;
use crate::localfs::backend::LocalFsBucket;
use crate::s3::backend::S3Bucket;
use crate::{
    CallbackWrapper, EnvironmentConfig, FileObject, FileObjectFilter,
    RowItemTrait, FileObjectVec, LakestreamError,
};

#[derive(Debug, Clone)]
pub enum ObjectStore {
    S3Bucket(S3Bucket),
    LocalFsBucket(LocalFsBucket),
}

impl ObjectStore {
    pub fn new(
        name: &str,
        config: EnvironmentConfig,
    ) -> Result<ObjectStore, String> {
        if name.starts_with("s3://") {
            let name = name.trim_start_matches("s3://");
            let bucket =
                S3Bucket::new(name, config).map_err(|err| err.to_string())?;
            Ok(ObjectStore::S3Bucket(bucket))
        } else if name.starts_with("localfs://") {
            let name = name.trim_start_matches("localfs://");
            let local_fs = LocalFsBucket::new(name, config)
                .map_err(|err| err.to_string())?;
            Ok(ObjectStore::LocalFsBucket(local_fs))
        } else {
            Err("Unsupported object store.".to_string())
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.name(),
            ObjectStore::LocalFsBucket(local_fs) => local_fs.name(),
        }
    }

    pub fn config(&self) -> &EnvironmentConfig {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.config(),
            ObjectStore::LocalFsBucket(local_fs) => local_fs.config(),
        }
    }

    pub fn println_path(&self) -> String {
        match self {
            ObjectStore::S3Bucket(bucket) => {
                format!("s3://{}", bucket.name())
            }
            ObjectStore::LocalFsBucket(local_fs) => {
                format!("{}", local_fs.name())
            }
        }
    }

    pub async fn list_files(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
    ) -> Result<Vec<FileObject>, LakestreamError> {
        let mut file_objects = FileObjectVec::new(None);
        match self {
            ObjectStore::S3Bucket(bucket) => {
                bucket
                    .list_files(
                        prefix,
                        recursive,
                        max_keys,
                        filter,
                        &mut file_objects,
                    )
                    .await
            }
            ObjectStore::LocalFsBucket(local_fs) => {
                local_fs
                    .list_files(
                        prefix,
                        recursive,
                        max_keys,
                        filter,
                        &mut file_objects,
                    )
                    .await
            }
        }?;
        Ok(file_objects.into_inner())
    }

    pub async fn list_files_with_callback(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: CallbackWrapper<FileObject>,
    ) -> Result<(), LakestreamError> {
        let callback = match callback {
            CallbackWrapper::Sync(sync_callback) => {
                Some(Box::new(move |file_objects: &[FileObject]| {
                    sync_callback(file_objects);
                    Box::pin(futures::future::ready(()))
                        as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
                })
                    as Box<
                        dyn Fn(
                                &[FileObject],
                            ) -> Pin<
                                Box<dyn Future<Output = ()> + Send + 'static>,
                            > + Send
                            + Sync,
                    >)
            }
            CallbackWrapper::Async(async_callback) => {
                Some(Box::new(move |file_objects: &[FileObject]| {
                    Box::pin(async_callback(file_objects.to_vec()))
                        as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
                })
                    as Box<
                        dyn Fn(
                                &[FileObject],
                            ) -> Pin<
                                Box<dyn Future<Output = ()> + Send + 'static>,
                            > + Send
                            + Sync,
                    >)
            }
        };

        let mut file_objects = FileObjectVec::new(callback);
        match self {
            ObjectStore::S3Bucket(bucket) => {
                bucket
                    .list_files(
                        prefix,
                        recursive,
                        max_files,
                        filter,
                        &mut file_objects,
                    )
                    .await
            }
            ObjectStore::LocalFsBucket(local_fs) => {
                local_fs
                    .list_files(
                        prefix,
                        recursive,
                        max_files,
                        filter,
                        &mut file_objects,
                    )
                    .await
            }
        }
    }

    pub async fn get_object(
        &self,
        key: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), LakestreamError> {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.get_object(key, data).await,
            ObjectStore::LocalFsBucket(local_fs) => {
                local_fs.get_object(key, data).await
            }
        }
    }
}

impl CallbackItem for ObjectStore {
    fn println_path(&self) -> String {
        self.println_path()
    }
}

impl RowItemTrait for ObjectStore {
    fn name(&self) -> &str {
        self.name()
    }
    fn println_path(&self) -> String {
        self.println_path()
    }
}


#[async_trait(?Send)]
pub trait ObjectStoreTrait: Send {
    fn name(&self) -> &str;
    fn config(&self) -> &EnvironmentConfig;
    async fn list_files(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
        file_objects: &mut FileObjectVec, // Change this parameter
    ) -> Result<(), LakestreamError>;
    async fn get_object(
        &self,
        key: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), LakestreamError>;
    async fn head_object(
        &self,
        key: &str,
    ) -> Result<(u16, HashMap<String, String>), LakestreamError>;
}
