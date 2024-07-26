use async_trait::async_trait;

pub use super::bucket::LocalFsBucket;
use crate::handlers::object_store::ObjectStoreBackend;
use crate::{EnvironmentConfig, InternalError, ObjectStoreTable};

pub struct LocalFsBackend;

#[async_trait(?Send)]
impl ObjectStoreBackend for LocalFsBackend {
    fn new(_config: EnvironmentConfig) -> Result<Self, InternalError> {
        Ok(Self)
    }

    async fn list_buckets(
        _config: EnvironmentConfig,
        _max_files: Option<u32>,
        _table: &mut ObjectStoreTable,
    ) -> Result<(), InternalError> {
        Ok(())
    }
}
