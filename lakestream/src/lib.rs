pub(crate) mod base;
pub(crate) mod default;
pub(crate) mod http;
pub(crate) mod localfs;
pub(crate) mod s3;
pub(crate) mod utils;

pub use base::cli;
pub use base::interfaces::{
    FileObject, ListObjectsResult, ObjectStore, ObjectStoreHandler,
};
pub use default::DEFAULT_AWS_REGION;
