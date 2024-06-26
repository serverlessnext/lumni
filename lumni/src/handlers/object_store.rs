use core::panic;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use log::debug;
use sqlparser::ast::{Expr, Query, SelectItem, SetExpr, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::localfs::backend::LocalFsBucket;
use crate::s3::backend::S3Bucket;
use crate::table::object_store::table_from_list_bucket;
use crate::table::{FileObjectTable, Table, TableCallback};
use crate::{
    BinaryCallbackWrapper, EnvironmentConfig, FileObjectFilter,
    LakestreamError, ObjectStoreTable, ParsedUri, UriScheme,
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
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        let mut table = FileObjectTable::new(&selected_columns, callback);

        match self {
            ObjectStore::S3Bucket(bucket) => {
                bucket
                    .list_files(
                        prefix,
                        selected_columns,
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
    ) -> Result<(), LakestreamError> {
        match self {
            ObjectStore::S3Bucket(bucket) => bucket.get_object(key, data).await,
            ObjectStore::LocalFsBucket(local_fs) => {
                local_fs.get_object(key, data).await
            }
        }
    }
}

#[async_trait(?Send)]
pub trait ObjectStoreTrait: Send {
    fn name(&self) -> &str;
    fn config(&self) -> &EnvironmentConfig;
    async fn list_files(
        &self,
        prefix: Option<&str>,
        selected_columns: &Option<Vec<&str>>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
        table: &mut FileObjectTable,
    ) -> Result<(), LakestreamError>;
    async fn get_object(
        &self,
        key: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), LakestreamError>;
    async fn head_object(
        &self,
        key: &str,
    ) -> Result<(u16, HashMap<String, String>), LakestreamError>;
}

#[derive(Clone)]
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
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        if let Some(bucket) = &parsed_uri.bucket {
            // list files in a bucket
            debug!("Listing files in bucket {}", bucket);
            let table = self
                .list_files_in_bucket(
                    parsed_uri, // FROM
                    config.clone(),
                    &selected_columns, // SELECT
                    recursive,         // true in case Query is used
                    max_files,         // LIMIT
                    filter,            // WHERE
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
            Err(LakestreamError::NoBucketInUri(parsed_uri.to_string()))
        }
    }

    pub async fn list_buckets(
        &self,
        parsed_uri: &ParsedUri,
        config: &EnvironmentConfig,
        selected_columns: &Option<Vec<&str>>,
        max_files: Option<u32>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
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
    ) -> Result<Option<Vec<u8>>, LakestreamError> {
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
            Err(LakestreamError::NoBucketInUri(parsed_uri.to_string()))
        }
    }

    async fn list_files_in_bucket(
        &self,
        parsed_uri: &ParsedUri,
        config: EnvironmentConfig,
        selected_columns: &Option<Vec<&str>>,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
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
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        let dialect = GenericDialect {};
        let parsed = Parser::parse_sql(&dialect, statement);

        match parsed {
            Ok(statements) => {
                if let Some(Statement::Query(query)) =
                    statements.into_iter().next()
                {
                    self.handle_select_statement(&query, config, callback).await
                } else {
                    Err(LakestreamError::InternalError(
                        "Unsupported query statement".to_string(),
                    ))
                }
            }
            Err(_e) => Err(LakestreamError::InternalError(
                "Failed to parse query statement".to_string(),
            )),
        }
    }

    async fn handle_select_statement(
        &self,
        query: &Query,
        config: &EnvironmentConfig,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        if let SetExpr::Select(select) = &*query.body {
            let selected_columns = if select
                .projection
                .iter()
                .any(|item| matches!(item, SelectItem::Wildcard(_)))
            {
                // wildcard: e.g. SELECT * FROM "uri"
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
                // assume the query is of the form 'SELECT * FROM "uri"'
                // TODOs:
                // 1. directories vs files
                // in this first implementation, everything is treated as a directory,
                // while we should distinguish between files and directories
                // directories -> call list_objects()
                // files -> treat as a database file (e.g. .sql, .parquet)
                //
                // 2. handle the following SQL clauses:
                // non-wildcards in the SELECT statementA, e.g. 'SELECT name, size FROM "uri"'
                // WHERE clauses, e.g. 'SELECT * FROM "uri" WHERE size > 100'
                // ORDER BY, LIMIT and OFFSET clauses
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

                let result = self
                    .list_objects(
                        &ParsedUri::from_uri(&uri, true),
                        config,
                        selected_columns,
                        true,
                        limit,
                        &None,
                        callback.clone(),
                    )
                    .await;

                match result {
                    Err(LakestreamError::NoBucketInUri(_)) => {
                        // uri does not point to a bucket or (virtual) directory
                        // assume it to be a pointer to a database file (e.g. .sql, .parquet)
                        return self
                            .query_object(&uri, config, query, callback)
                            .await;
                    }
                    _ => return result, // TODO: query should return Table
                }
            }
        }
        //}

        Err(LakestreamError::InternalError(
            "Query does not match 'SELECT * FROM uri' pattern".to_string(),
        ))
    }

    async fn query_object(
        &self,
        _uri: &str,
        _config: &EnvironmentConfig,
        _query: &Query,
        _callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        // Logic to treat the URI as a database file and query it

        // This is a placeholder for the actual implementation.
        Err(LakestreamError::InternalError(
            "Querying object not implemented".to_string(),
        ))
    }
}

#[allow(dead_code)]
#[async_trait(?Send)]
pub trait ObjectStoreBackend: Send {
    fn new(config: EnvironmentConfig) -> Result<Self, LakestreamError>
    where
        Self: Sized;

    async fn list_buckets(
        config: EnvironmentConfig,
        max_files: Option<u32>,
        table: &mut ObjectStoreTable,
    ) -> Result<(), LakestreamError>;
}
