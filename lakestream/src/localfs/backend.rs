use async_trait::async_trait;

pub use super::bucket::{FileSystem, LocalFsBucket};
use crate::{Config, LakestreamError, ObjectStoreBackend, ObjectStoreVec};

pub struct LocalFsBackend;

#[async_trait(?Send)]
impl ObjectStoreBackend for LocalFsBackend {
    fn new(_config: Config) -> Result<Self, LakestreamError> {
        Ok(Self)
    }

    async fn list_buckets(
        _config: Config,
        _object_stores: &mut ObjectStoreVec,
    ) -> Result<(), LakestreamError> {
        Ok(())
    }
}
