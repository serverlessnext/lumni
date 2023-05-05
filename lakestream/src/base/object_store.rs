use std::ops::{Deref, DerefMut};
use std::pin::Pin;

use async_trait::async_trait;
use futures::Future;
use log::error;

use crate::api::object_store_handler::ObjectStoreBackend;
use crate::base::callback_wrapper::CallbackItem;
use crate::localfs::backend::{LocalFsBackend, LocalFsBucket};
use crate::s3::backend::{S3Backend, S3Bucket};
use crate::{
    CallbackWrapper, Config, FileObject, FileObjectFilter, FileObjectVec,
    LakestreamError,
};

type BoxedAsyncCallbackForObjectStore = Box<
    dyn Fn(&[ObjectStore]) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync
        + 'static,
>;

pub struct ObjectStoreVec {
    object_stores: Vec<ObjectStore>,
    callback: Option<BoxedAsyncCallbackForObjectStore>,
}

impl ObjectStoreVec {
    pub fn new(callback: Option<BoxedAsyncCallbackForObjectStore>) -> Self {
        Self {
            object_stores: Vec::new(),
            callback,
        }
    }

    pub fn into_inner(self) -> Vec<ObjectStore> {
        self.object_stores
    }

    pub async fn extend_async<T: IntoIterator<Item = ObjectStore>>(
        &mut self,
        iter: T,
    ) {
        let new_object_stores: Vec<ObjectStore> = iter.into_iter().collect();

        if let Some(callback) = &self.callback {
            log::info!("Calling callback for new object stores with callback");
            let fut = (callback)(&new_object_stores);
            fut.await;
        }

        self.object_stores.extend(new_object_stores);
    }
}

impl Deref for ObjectStoreVec {
    type Target = Vec<ObjectStore>;

    fn deref(&self) -> &Self::Target {
        &self.object_stores
    }
}

impl DerefMut for ObjectStoreVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object_stores
    }
}

#[derive(Clone)]
pub enum ObjectStore {
    S3Bucket(S3Bucket),
    LocalFsBucket(LocalFsBucket),
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

    pub fn config(&self) -> &Config {
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
}

impl CallbackItem for ObjectStore {
    fn println_path(&self) -> String {
        self.println_path()
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
        file_objects: &mut FileObjectVec, // Change this parameter
    ) -> Result<(), LakestreamError>;
}

pub async fn object_stores_from_config(
    config: Config,
    callback: &Option<CallbackWrapper<ObjectStore>>,
) -> Result<ObjectStoreVec, LakestreamError> {
    let uri = config.get("uri").unwrap_or(&"".to_string()).clone();

    let callback = match callback {
        Some(CallbackWrapper::Sync(sync_callback)) => {
            let sync_callback = sync_callback.clone();
            Some(Box::new(move |object_stores: &[ObjectStore]| {
                sync_callback(object_stores);
                Box::pin(futures::future::ready(()))
                    as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            }) as BoxedAsyncCallbackForObjectStore)
        }

        Some(CallbackWrapper::Async(async_callback)) => {
            let async_callback = async_callback.clone();
            Some(Box::new(move |object_stores: &[ObjectStore]| {
                Box::pin(async_callback(object_stores.to_vec()))
                    as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            }) as BoxedAsyncCallbackForObjectStore)
        }
        None => None,
    };

    let mut object_stores = ObjectStoreVec::new(callback);

    if uri.starts_with("s3://") {
        // Delegate the logic to the S3 backend
        S3Backend::list_buckets(config.clone(), &mut object_stores).await?;
    } else if uri.starts_with("localfs://") {
        // Delegate the logic to the LocalFs backend
        LocalFsBackend::list_buckets(config.clone(), &mut object_stores)
            .await?;
    } else {
        error!("Unsupported object store type: {}", uri);
    }

    Ok(object_stores)
}
