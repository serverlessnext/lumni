pub(crate) mod api;
pub(crate) mod base;
pub(crate) mod handlers;
pub(crate) mod default;
pub(crate) mod error;
pub(crate) mod http;
pub(crate) mod localfs;
pub(crate) mod s3;
pub(crate) mod table;
pub(crate) mod utils;

pub use api::object_store_handler::{ObjectStoreBackend, ObjectStoreHandler};
pub use base::callback_wrapper::{
    BinaryCallbackWrapper, CallbackItem, CallbackWrapper,
};
pub use base::config::EnvironmentConfig;
pub use base::file_object::FileObject;
pub use base::filters::FileObjectFilter;
pub use base::object_store::{ObjectStore, ObjectStoreTrait};
pub use handlers::HttpHandler;

// re-export all defaults
pub use default::*;
pub use error::LakestreamError;
pub use table::{
    FileObjectTable, ObjectStoreTable, Table, TableCallback, TableColumn,
    TableColumnValue, TableRow,
};
pub use utils::formatters;

// cli-specific modules
pub(crate) mod parser;
pub(crate) mod subcommands;
pub use parser::run_cli;
