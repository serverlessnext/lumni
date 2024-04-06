pub(crate) mod api;
pub(crate) mod base;
pub(crate) mod default;
pub(crate) mod error;
pub(crate) mod handlers;
pub(crate) mod http;
pub(crate) mod localfs;
pub(crate) mod s3;
pub(crate) mod table;
pub(crate) mod utils;

pub use base::callback_wrapper::{
    BinaryCallbackWrapper, CallbackItem, CallbackWrapper,
};
pub use base::config::EnvironmentConfig;
pub use base::file_object::FileObject;
pub use base::filters::FileObjectFilter;
// re-export all defaults
pub use default::*;
pub use error::LakestreamError;
#[cfg(feature = "http_client")]
pub use handlers::HttpHandler;
pub use handlers::ObjectStoreHandler;
pub use table::{
    FileObjectTable, ObjectStoreTable, Table, TableCallback, TableColumn,
    TableColumnValue, TableRow,
};
pub use utils::{ParsedUri, UriScheme};
