
use std::pin::Pin;
use std::future::Future;
use std::ops::{Deref, DerefMut};

use crate::base::object_store::BoxedAsyncCallbackForObjectStore;
use crate::localfs::backend::LocalFsBackend;
use crate::s3::backend::S3Backend;

use crate::{
    CallbackWrapper, Config,
    ObjectStore, ObjectStoreVec, LakestreamError,
};


pub async fn object_stores_from_config(
    config: Config,
    callback: &Option<CallbackWrapper<ObjectStore>>,
) -> Result<ObjectStoreVec, LakestreamError> {
    let uri = config.get("uri").unwrap_or(&"".to_string()).clone();

    let callback = match callback {
        Some(CallbackWrapper::Sync(sync_callback)) => {
            Some(Box::new(move |object_stores: &[ObjectStore]| {
                let sync_callback = sync_callback.clone();
                sync_callback(object_stores);
                Box::pin(futures::future::ready(()))
                    as Pin<Box<dyn Future<Output = ()> + Send>>
            }) as BoxedAsyncCallbackForObjectStore)
        }

        Some(CallbackWrapper::Async(async_callback)) => {
            let async_callback = async_callback.clone();
            Some(Box::new(move |object_stores: &[ObjectStore]| {
                Box::pin(async_callback(object_stores.to_vec()))
                    as Pin<Box<dyn Future<Output = ()> + Send>>
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
        LocalFsBackend::list_buckets(config.clone(), &mut object_stores).await?;
    } else {
        error!("Unsupported object store type: {}", uri);
    }

    Ok(object_stores)
}

