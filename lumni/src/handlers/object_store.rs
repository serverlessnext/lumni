use core::panic;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use log::debug;
use sqlparser::ast::{Expr, Query, SelectItem, SetExpr, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::api::error::{LumniError, ResourceError};
use crate::localfs::backend::LocalFsBucket;
use crate::s3::backend::S3Bucket;
use crate::table::object_store::table_from_list_bucket;
use crate::table::{FileObjectTable, Table, TableCallback};
use crate::{
    BinaryCallbackWrapper, EnvironmentConfig, FileObjectFilter, IgnoreContents,
    InternalError, ObjectStoreTable, ParsedUri, UriScheme,
};

#[derive(Debug, Clone)]
pub enum ObjectStore {
    S3Bucket(S3Bucket),
    LocalFsBucket(LocalFsBucket),
}

impl ObjectStore {
    pub fn new(
        name: &str,
        config: EnvironmentConfig,
    ) -> Result<ObjectStore, String> {
        if name.starts_with("s3://") {
            let name = name.trim_start_matches("s3://");
            let bucket =
                S3Bucket::new(name, config).map_err(|err| err.to_string())?;
            Ok(ObjectStore::S3Bucket(bucket))
        } else if name.starts_with("localfs://") {
            let name = name.trim_start_matches("localfs://");
            let local_fs = LocalFsBucket::new(name, config)
                .map_err(|err| err.to_string())?;
            Ok(ObjectStore::LocalFsBucket(local_fs))
        } else {
            // add name to error message
            let err_msg = format!("Unsupported object store: {}", name);
            Err(err_msg)
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.name(),
            ObjectStore::LocalFsBucket(local_fs) => local_fs.name(),
        }
    }

    pub fn config(&self) -> &EnvironmentConfig {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.config(),
            ObjectStore::LocalFsBucket(local_fs) => local_fs.config(),
        }
    }

    pub fn uri(&self) -> String {
        match self {
            ObjectStore::S3Bucket(bucket) => {
                format!("s3://{}", bucket.name())
            }
            ObjectStore::LocalFsBucket(local_fs) => {
                format!("{}", local_fs.name())
            }
        }
    }

    pub async fn list_files(
        &self,
        prefix: Option<&str>,
        selected_columns: &Option<Vec<&str>>,
        skip_hidden: bool,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, InternalError> {
        let mut table = FileObjectTable::new(&selected_columns, callback);

        match self {
            ObjectStore::S3Bucket(bucket) => {
                bucket
                    .list_files(
                        prefix,
                        selected_columns,
                        skip_hidden,
                        recursive,
                        max_files,
                        filter,
                        &mut table,
                    )
                    .await
            }
            ObjectStore::LocalFsBucket(local_fs) => {
                local_fs
                    .list_files(
                        prefix,
                        selected_columns,
                        skip_hidden,
                        recursive,
                        max_files,
                        filter,
                        &mut table,
                    )
                    .await
            }
        }?;
        Ok(Box::new(table))
    }

    pub async fn get_object(
        &self,
        key: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), InternalError> {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.get_object(key, data).await,
            ObjectStore::LocalFsBucket(local_fs) => {
                local_fs.get_object(key, data).await
            }
        }
    }

    pub async fn head_object(
        &self,
        key: &str,
    ) -> Result<(u16, HashMap<String, String>), InternalError> {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.head_object(key).await,
            ObjectStore::LocalFsBucket(local_fs) => {
                local_fs.head_object(key).await
            }
        }
    }
}

#[async_trait(?Send)]
pub trait ObjectStoreTrait: Send + Sync {
    fn name(&self) -> &str;
    fn config(&self) -> &EnvironmentConfig;
    async fn list_files(
        &self,
        prefix: Option<&str>,
        selected_columns: &Option<Vec<&str>>,
        skip_hidden: bool,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
        table: &mut FileObjectTable,
    ) -> Result<(), InternalError>;
    async fn get_object(
        &self,
        key: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), InternalError>;
    async fn head_object(
        &self,
        key: &str,
    ) -> Result<(u16, HashMap<String, String>), InternalError>;
}

#[derive(Debug, Clone)]
pub struct ObjectStoreHandler {}

impl ObjectStoreHandler {
    pub fn new(_configs: Option<Vec<EnvironmentConfig>>) -> Self {
        // creating with config will be used in future
        ObjectStoreHandler {}
    }

    pub async fn list_objects(
        &self,
        parsed_uri: &ParsedUri,
        config: &EnvironmentConfig,
        selected_columns: Option<Vec<&str>>,
        skip_hidden: bool,
        recursive: bool,
        max_files: Option<u32>,
        filter: Option<FileObjectFilter>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, InternalError> {
        if let Some(bucket) = &parsed_uri.bucket {
            // list files in a bucket
            debug!("Listing files in bucket {}", bucket);
            let table = self
                .list_files_in_bucket(
                    parsed_uri, // FROM
                    config.clone(),
                    &selected_columns, // SELECT
                    skip_hidden,       // skip_hidden
                    recursive,         // true in case Query is used
                    max_files,         // LIMIT
                    &filter,           // WHERE
                    callback,          // callback is a custom function
                                       // applied to what gets selected (via ROW addition)
                )
                .await?;
            Ok(table)
        } else {
            if parsed_uri.scheme == UriScheme::S3 {
                debug!("Listing buckets on S3");
                return self
                    .list_buckets(
                        &parsed_uri,
                        config,
                        &selected_columns,
                        max_files,
                        callback,
                    )
                    .await;
            }
            Err(InternalError::NoBucketInUri(parsed_uri.to_string()))
        }
    }

    pub async fn list_buckets(
        &self,
        parsed_uri: &ParsedUri,
        config: &EnvironmentConfig,
        selected_columns: &Option<Vec<&str>>,
        max_files: Option<u32>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, InternalError> {
        if parsed_uri.bucket.is_some() {
            // should not happen, prefer to panic in case it does
            panic!("list_buckets called with a bucket uri");
        }

        // Clone the original config and update the settings
        let mut updated_config = config.clone();
        updated_config.set(
            "uri".to_string(),
            format!("{}://", parsed_uri.scheme.to_string()),
        );
        table_from_list_bucket(
            updated_config,
            selected_columns,
            max_files,
            callback,
        )
        .await
    }

    pub async fn get_object(
        &self,
        parsed_uri: &ParsedUri,
        config: &EnvironmentConfig,
        callback: Option<BinaryCallbackWrapper>,
    ) -> Result<Option<Vec<u8>>, InternalError> {
        if let Some(bucket) = &parsed_uri.bucket {
            let bucket_uri =
                format!("{}://{}", parsed_uri.scheme.to_string(), bucket);
            let key = parsed_uri.path.as_deref().unwrap_or("");
            let object_store = ObjectStore::new(&bucket_uri, config.clone())?;

            // NOTE: initial callback implementation for get_object. In future updates the callback
            // mechanism will be pushed to the underlying object store methods, so we can add
            // chunking as well for increased performance and ability to handle big files that not
            // fit in memory
            if let Some(callback) = callback {
                let mut data = Vec::new();
                object_store.get_object(key, &mut data).await?;
                callback.call(data).await?;
                Ok(None)
            } else {
                let mut data = Vec::new();
                object_store.get_object(key, &mut data).await?;
                Ok(Some(data))
            }
        } else {
            Err(InternalError::NoBucketInUri(parsed_uri.to_string()))
        }
    }

    async fn list_files_in_bucket(
        &self,
        parsed_uri: &ParsedUri,
        config: EnvironmentConfig,
        selected_columns: &Option<Vec<&str>>,
        skip_hidden: bool,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, InternalError> {
        let bucket_uri = format!(
            "{}://{}",
            parsed_uri.scheme.to_string(),
            parsed_uri.bucket.as_ref().unwrap()
        );
        let object_store = ObjectStore::new(&bucket_uri, config).unwrap();
        object_store
            .list_files(
                parsed_uri.path.as_deref(),
                selected_columns,
                skip_hidden,
                recursive,
                max_files,
                filter,
                callback,
            )
            .await
    }

    pub async fn execute_query(
        &self,
        statement: &str,
        config: &EnvironmentConfig,
        skip_hidden: bool,
        recursive: bool,
        ignore_contents: Option<IgnoreContents>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LumniError> {
        let dialect = GenericDialect {};
        let parsed = Parser::parse_sql(&dialect, statement);

        match parsed {
            Ok(statements) => {
                if let Some(Statement::Query(query)) =
                    statements.into_iter().next()
                {
                    let result = self
                        .handle_select_statement(
                            &query,
                            config,
                            skip_hidden,
                            recursive,
                            ignore_contents,
                            callback,
                        )
                        .await;

                    match result {
                        Ok(table) => Ok(table),
                        Err(InternalError::NotFound(e)) => Err(
                            LumniError::Resource(ResourceError::NotFound(e)),
                        ),
                        Err(e) => Err(LumniError::InternalError(e.to_string())),
                    }
                } else {
                    Err(LumniError::NotImplemented(
                        "Unsupported query statement".to_string(),
                    ))
                }
            }
            Err(_e) => Err(LumniError::ParseError(
                "Failed to parse query statement".to_string(),
            )),
        }
    }

    async fn handle_select_statement(
        &self,
        query: &Query,
        config: &EnvironmentConfig,
        skip_hidden: bool,
        recursive: bool,
        ignore_contents: Option<IgnoreContents>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, InternalError> {
        if let SetExpr::Select(select) = &*query.body {
            let selected_columns = if select
                .projection
                .iter()
                .any(|item| matches!(item, SelectItem::Wildcard(_)))
            {
                // wildcard: e.g. SELECT * FROM "uri"
                // localfiles: SELECT * FROM "."
                None
            } else {
                Some(
                    select
                        .projection
                        .iter()
                        .filter_map(|item| {
                            match item {
                                SelectItem::UnnamedExpr(Expr::Identifier(
                                    ident,
                                )) => Some(ident.value.as_str()), // Directly use the reference
                                _ => {
                                    log::warn!(
                                        "Skipping non-identifier selection: \
                                         {:?}",
                                        item
                                    );
                                    None
                                }
                            }
                        })
                        .collect::<Vec<&str>>(),
                ) // Collect as Vec<&str>
            };

            if let Some(table) = select.from.first() {
                let mut uri = table.relation.to_string();

                // check for and remove leading and trailing quotes
                if (uri.starts_with('"') && uri.ends_with('"'))
                    || (uri.starts_with('\'') && uri.ends_with('\''))
                {
                    uri = uri[1..uri.len() - 1].to_string(); // Remove the first and last characters
                }

                // Extract the LIMIT value if present
                let limit = match &query.limit {
                    Some(sqlparser::ast::Expr::Value(
                        sqlparser::ast::Value::Number(n, _),
                    )) => n.parse::<u32>().ok(),
                    _ => None,
                };

                let glob_matcher =
                    if let Some(ignore_contents) = ignore_contents {
                        ignore_contents.to_glob_matcher().map_err(|e| {
                            InternalError::ConfigError(e.to_string())
                        })?
                    } else {
                        None
                    };
                // Extract the WHERE clause if present
                let mut file_object_filter = match &select.selection {
                    Some(where_clause) => {
                        FileObjectFilter::parse_where_clause(where_clause)?
                    }
                    None => None,
                };

                if let Some(filter) = &mut file_object_filter {
                    // Update the glob_matcher in the filter if it exists
                    if let Some(glob_matcher) = &glob_matcher {
                        filter.glob_matcher = Some(glob_matcher.clone());
                    }
                } else if let Some(glob_matcher) = &glob_matcher {
                    // Create a new filter if it doesn't exist
                    file_object_filter = Some(FileObjectFilter {
                        conditions: vec![],
                        glob_matcher: Some(glob_matcher.clone()),
                        include_directories: true,
                    });
                };

                // TODO: handle ORDER BY and OFFSET clauses

                let result = self
                    .list_objects(
                        &ParsedUri::from_uri(&uri, true),
                        config,
                        selected_columns,
                        skip_hidden,
                        recursive,
                        limit,
                        file_object_filter,
                        callback.clone(),
                    )
                    .await;
                match result {
                    Err(InternalError::NoBucketInUri(_)) => {
                        // uri does not point to a bucket or (virtual) directory
                        // assume it to be a pointer to a database file (e.g. .sql, .parquet)
                        return self
                            .query_object(
                                &ParsedUri::from_uri(&uri, true),
                                config,
                                query,
                                callback,
                            )
                            .await;
                    }
                    _ => return result, // TODO: query should return Table
                }
            }
        }

        Err(InternalError::InternalError(
            "Query does not match 'SELECT * FROM uri' pattern".to_string(),
        ))
    }

    async fn query_object(
        &self,
        parsed_uri: &ParsedUri,
        config: &EnvironmentConfig,
        _query: &Query,
        _callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, InternalError> {
        // Logic to treat the URI as a database file and query it
        let object_store =
            ObjectStore::new(&parsed_uri.to_string(), config.clone())?;
        let _object_data = object_store
            .head_object(parsed_uri.path.as_deref().unwrap_or(""))
            .await?;

        // TODO: return file object_data as Table
        let table = FileObjectTable::new(&None, None);
        Ok(Box::new(table))
    }
}

#[allow(dead_code)]
#[async_trait(?Send)]
pub trait ObjectStoreBackend: Send {
    fn new(config: EnvironmentConfig) -> Result<Self, InternalError>
    where
        Self: Sized;

    async fn list_buckets(
        config: EnvironmentConfig,
        max_files: Option<u32>,
        table: &mut ObjectStoreTable,
    ) -> Result<(), InternalError>;
}
