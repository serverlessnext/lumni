use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::vec::Vec;

use log::error;

use crate::api::object_store_handler::ObjectStoreBackend;
use crate::localfs::backend::LocalFsBackend;
use crate::s3::backend::S3Backend;
use crate::{
    CallbackItem, CallbackWrapper, EnvironmentConfig, FileObject,
    LakestreamError, ObjectStore,
};

pub type BoxedAsyncCallbackForRowItem = Box<
    dyn Fn(&[RowItem]) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync
        + 'static,
>;
pub trait RowItemTrait {
    fn name(&self) -> &str;
    fn println_path(&self) -> String;
}

#[derive(Debug, Clone)]
pub enum RowType {
    ObjectStore(ObjectStore),
    FileObject(FileObject),
}

impl RowType {
    pub fn name(&self) -> &str {
        match self {
            RowType::ObjectStore(obj) => obj.name(),
            RowType::FileObject(obj) => obj.name(),
        }
    }

    pub fn println_path(&self) -> String {
        match self {
            RowType::ObjectStore(obj) => obj.println_path(),
            RowType::FileObject(obj) => obj.println_path(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RowItem {
    row_type: RowType,
}

impl RowItem {
    pub fn new(row_type: RowType) -> Self {
        Self { row_type }
    }

    pub fn name(&self) -> &str {
        self.row_type.name()
    }

    pub fn println_path(&self) -> String {
        self.row_type.println_path()
    }
}

pub struct RowItemVec {
    items: Vec<RowItem>,
    callback: Option<BoxedAsyncCallbackForRowItem>,
}

impl RowItemVec {
    pub fn new(callback: Option<BoxedAsyncCallbackForRowItem>) -> Self {
        Self {
            items: Vec::new(),
            callback,
        }
    }

    pub fn into_inner(self) -> Vec<RowItem> {
        self.items
    }

    pub async fn extend_async<T: IntoIterator<Item = RowItem>>(
        &mut self,
        iter: T,
    ) {
        let new_items: Vec<RowItem> = iter.into_iter().collect();

        if let Some(callback) = &self.callback {
            let fut = (callback)(&new_items);
            fut.await;
        }

        self.items.extend(new_items);
    }
}

impl Deref for RowItemVec {
    type Target = Vec<RowItem>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for RowItemVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl CallbackItem for RowItem {
    fn println_path(&self) -> String {
        self.println_path()
    }
}

pub async fn row_items_from_list_bucket(
    config: EnvironmentConfig,
    callback: &Option<CallbackWrapper<RowItem>>,
) -> Result<RowItemVec, LakestreamError> {
    let uri = config.get("uri").unwrap_or(&"".to_string()).clone();

    let callback = match callback {
        Some(CallbackWrapper::Sync(sync_callback)) => {
            let sync_callback = sync_callback.clone();
            Some(Box::new(move |row_items: &[RowItem]| {
                sync_callback(row_items);
                Box::pin(futures::future::ready(()))
                    as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            }) as BoxedAsyncCallbackForRowItem)
        }

        Some(CallbackWrapper::Async(async_callback)) => {
            let async_callback = async_callback.clone();
            Some(Box::new(move |row_items: &[RowItem]| {
                Box::pin(async_callback(row_items.to_vec()))
                    as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            }) as BoxedAsyncCallbackForRowItem)
        }
        None => None,
    };

    let mut row_items_vec = RowItemVec::new(callback);

    if uri.starts_with("s3://") {
        // Delegate the logic to the S3 backend
        S3Backend::list_buckets(config.clone(), &mut row_items_vec).await?;
    } else if uri.starts_with("localfs://") {
        // Delegate the logic to the LocalFs backend
        LocalFsBackend::list_buckets(config.clone(), &mut row_items_vec)
            .await?;
    } else {
        error!("Unsupported object store type: {}", uri);
    }

    Ok(row_items_vec)
}
