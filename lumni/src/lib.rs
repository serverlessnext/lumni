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
pub use api::error::{ApplicationError, LumniError};
pub use base::callback_wrapper::{
    BinaryCallbackWrapper, CallbackItem, CallbackWrapper,
};
pub use base::config::EnvironmentConfig;
pub use base::file_object::FileObject;
pub use base::{FileObjectFilter, IgnoreContents};
// InternalError should be phased out in favor of LumniError
pub use error::InternalError;
pub use handlers::ObjectStoreHandler;
pub use table::{
    FileObjectTable, ObjectStoreTable, Table, TableCallback, TableColumn,
    TableColumnValue, TableRow,
};
pub use utils::{ParsedUri, UriScheme};

// meant for external use by third-party apps or libraries
pub mod external {
    pub use crate::apps::api;
    // generic
    pub use crate::base::config::EnvironmentConfig;
    #[cfg(feature = "http_client")]
    pub use crate::handlers::HttpHandler;
    pub use crate::handlers::ObjectStoreHandler;
    #[cfg(feature = "http_client")]
    pub use crate::http::client::{
        HttpClient, HttpClientError, HttpClientErrorHandler,
        HttpClientResponse, HttpClientResult,
    };
    #[cfg(feature = "http_client")]
    pub use crate::s3::{AWSCredentials, AWSRequestBuilder};
    pub use crate::table::{
        FileObjectTable, ObjectStoreTable, Table, TableCallback, TableColumn,
        TableColumnValue, TableRow,
    };
    pub use crate::utils::timestamp::Timestamp;
}
pub use default::*;
pub use external::*;
