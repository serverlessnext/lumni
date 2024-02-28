use std::sync::Arc;

use async_trait::async_trait;
use log::debug;
use sqlparser::ast::{Expr, Query, SelectItem, SetExpr, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::table::object_store::table_from_list_bucket;
use crate::utils::uri_parse::ParsedUri;
use crate::{
    BinaryCallbackWrapper, EnvironmentConfig, FileObjectFilter,
    LakestreamError, ObjectStore, ObjectStoreTable, Table, TableCallback,
};

#[derive(Clone)]
pub struct ObjectStoreHandler {}

impl ObjectStoreHandler {
    pub fn new(_configs: Option<Vec<EnvironmentConfig>>) -> Self {
        // creating with config will be used in future
        ObjectStoreHandler {}
    }

    pub async fn list_objects(
        &self,
        uri: &str,
        config: &EnvironmentConfig,
        selected_columns: Option<Vec<&str>>,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(uri, true);

        if let Some(bucket) = &parsed_uri.bucket {
            // list files in a bucket
            debug!("Listing files in bucket {}", bucket);
            let table = self.list_files_in_bucket(
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
            if let Some(scheme) = &parsed_uri.scheme {
                if scheme == "s3" {
                    debug!("Listing buckets on S3");
                    return self.list_buckets(uri, config, &selected_columns, max_files, callback).await
                }
            }
            Err(LakestreamError::NoBucketInUri(uri.to_string()))
        }
    }

    pub async fn list_buckets(
        &self,
        uri: &str,
        config: &EnvironmentConfig,
        selected_columns: &Option<Vec<&str>>,
        max_files: Option<u32>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(uri, true);

        if let Some(_) = &parsed_uri.bucket {
            return Err(LakestreamError::NoBucketInUri(uri.to_string()));
        }
        // Clone the original config and update the settings
        let mut updated_config = config.clone();
        updated_config.settings.insert(
            "uri".to_string(),
            format!("{}://", parsed_uri.scheme.unwrap()),
        );
        table_from_list_bucket(updated_config, selected_columns, max_files, callback).await
    }

    pub async fn get_object(
        &self,
        uri: &str,
        config: &EnvironmentConfig,
        callback: Option<BinaryCallbackWrapper>,
    ) -> Result<Option<Vec<u8>>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(uri, false);

        if let Some(bucket) = &parsed_uri.bucket {
            // Get the object from the bucket
            let bucket_uri = if let Some(scheme) = &parsed_uri.scheme {
                format!("{}://{}", scheme, bucket)
            } else {
                format!("localfs://{}", bucket)
            };

            let key = parsed_uri.path.as_deref().unwrap();
            let object_store =
                ObjectStore::new(&bucket_uri, config.clone()).unwrap();

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
            Err(LakestreamError::NoBucketInUri(uri.to_string()))
        }
    }

    async fn list_files_in_bucket(
        &self,
        parsed_uri: ParsedUri,
        config: EnvironmentConfig,
        selected_columns: &Option<Vec<&str>>,
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Result<Box<dyn Table>, LakestreamError> {
        let bucket_uri = if let Some(scheme) = &parsed_uri.scheme {
            format!("{}://{}", scheme, parsed_uri.bucket.as_ref().unwrap())
        } else {
            format!("localfs://{}", parsed_uri.bucket.as_ref().unwrap())
        };

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
    ) -> Result<(), LakestreamError> {
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
    ) -> Result<(), LakestreamError> {
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
                        &uri,
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
                    _ => return Ok(()), // TODO: query should return Table
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
    ) -> Result<(), LakestreamError> {
        // Logic to treat the URI as a database file and query it

        // This is a placeholder for the actual implementation.
        Err(LakestreamError::InternalError(
            "Querying object not implemented".to_string(),
        ))
    }
}

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
