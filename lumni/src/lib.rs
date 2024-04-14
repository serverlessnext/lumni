pub(crate) mod apps;
pub(crate) mod base;
pub(crate) mod default;
pub(crate) mod error;
pub(crate) mod handlers;
pub(crate) mod http;
pub(crate) mod localfs;
pub(crate) mod s3;
pub(crate) mod table;
pub(crate) mod utils;

// note - please not rely on these components to remain exposed as part of the API
// see external module for parts that are meant to be part of the stable API
pub use base::callback_wrapper::{
    BinaryCallbackWrapper, CallbackItem, CallbackWrapper,
};
pub use base::config::EnvironmentConfig;
pub use base::file_object::FileObject;
pub use base::filters::FileObjectFilter;
pub use error::LakestreamError;
pub use handlers::ObjectStoreHandler;
pub use table::{
    FileObjectTable, ObjectStoreTable, Table, TableCallback, TableColumn,
    TableColumnValue, TableRow,
};
pub use utils::{ParsedUri, UriScheme};

// meant for external use by third-party apps or libraries
pub mod external {
    pub use crate::apps::api;
    #[cfg(feature = "http_client")]
    pub use crate::http::HttpClient;
    #[cfg(feature = "http_client")]
    pub use crate::handlers::HttpHandler;
}
pub use external::*;
pub use default::*;