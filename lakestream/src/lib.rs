pub(crate) mod base;
pub(crate) mod cli;
pub(crate) mod default;
pub(crate) mod http;
pub(crate) mod localfs;
pub(crate) mod s3;
pub(crate) mod utils;
pub(crate) mod error;

pub use error::LakestreamError;
pub use base::filters::FileObjectFilter;
pub use base::interfaces::{
    FileObject, ListObjectsResult, ObjectStore, ObjectStoreHandler,
};
pub use cli::parser::run_cli;
// re-export all defaults
pub use default::*;
