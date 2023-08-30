use async_trait::async_trait;

pub use super::bucket::{FileSystem, LocalFsBucket};
use crate::{
    EnvironmentConfig, LakestreamError, ObjectStoreBackend, ObjectStoreVec,
};

pub struct LocalFsBackend;

#[async_trait(?Send)]
impl ObjectStoreBackend for LocalFsBackend {
    fn new(_config: EnvironmentConfig) -> Result<Self, LakestreamError> {
        Ok(Self)
    }

    async fn list_buckets(
        _config: EnvironmentConfig,
        _object_stores: &mut ObjectStoreVec,
    ) -> Result<(), LakestreamError> {
        Ok(())
    }
}
