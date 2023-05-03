use async_trait::async_trait;

pub use super::bucket::{FileSystem, LocalFsBucket};
use crate::{Config, LakestreamError, ObjectStore, ObjectStoreBackend};

pub struct LocalFsBackend;

#[async_trait]
impl ObjectStoreBackend for LocalFsBackend {
    fn new(_config: Config) -> Result<Self, LakestreamError> {
        Ok(Self)
    }

    async fn list_buckets(
        _config: Config,
    ) -> Result<Vec<ObjectStore>, LakestreamError> {
        Ok(vec![])
    }
}
