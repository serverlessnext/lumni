use log::error;

use crate::api::object_store_handler::ObjectStoreBackend;
use crate::localfs::backend::LocalFsBackend;
use crate::s3::backend::S3Backend;
use crate::{Config, LakestreamError, ObjectStore};

pub async fn object_stores_from_config(
    config: Config,
) -> Result<Vec<ObjectStore>, LakestreamError> {
    let uri = config.get("uri").unwrap_or(&"".to_string()).clone();
    if uri.starts_with("s3://") {
        // Delegate the logic to the S3 backend
        S3Backend::list_buckets(config).await
    } else if uri.starts_with("localfs://") {
        // Delegate the logic to the LocalFs backend
        LocalFsBackend::list_buckets(config).await
    } else {
        error!("Unsupported object store type: {}", uri);
        Ok(Vec::new())
    }
}
