use async_trait::async_trait;
use log::info;
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::base::row_item::row_items_from_list_bucket;
use crate::utils::uri_parse::ParsedUri;
use crate::{
    BinaryCallbackWrapper, CallbackWrapper, EnvironmentConfig, FileObject,
    FileObjectFilter, LakestreamError, ListObjectsResult, ObjectStore, RowItem,
    RowItemVec,
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
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<CallbackWrapper<FileObject>>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(uri, true);

        if let Some(bucket) = &parsed_uri.bucket {
            // list files in a bucket
            info!("Listing files in bucket {}", bucket);
            self.list_files_in_bucket(
                parsed_uri,
                config.clone(),
                recursive,
                max_files,
                filter,
                callback,
            )
            .await
        } else {
            Err(LakestreamError::NoBucketInUri(uri.to_string()))
        }
    }

    pub async fn list_buckets(
        &self,
        uri: &str,
        config: &EnvironmentConfig,
        callback: Option<CallbackWrapper<RowItem>>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        let parsed_uri = ParsedUri::from_uri(uri, true);

        if let Some(_) = &parsed_uri.bucket {
            return Err(LakestreamError::NoBucketInUri(uri.to_string()));
        }
        // list buckets
        // Clone the original config and update the settings
        // will change the input config to reference at future update
        let mut updated_config = config.clone();
        updated_config.settings.insert(
            "uri".to_string(),
            format!("{}://", parsed_uri.scheme.unwrap()),
        );

        let row_items =
            row_items_from_list_bucket(updated_config, &callback).await?;

        if callback.is_some() {
            // callback used, so can just return None
            Ok(None)
        } else {
            // no callback used -- return as list of row items
            Ok(Some(ListObjectsResult::RowItems(row_items.into_inner())))
        }
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
        recursive: bool,
        max_files: Option<u32>,
        filter: &Option<FileObjectFilter>,
        callback: Option<CallbackWrapper<FileObject>>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        let bucket_uri = if let Some(scheme) = &parsed_uri.scheme {
            format!("{}://{}", scheme, parsed_uri.bucket.as_ref().unwrap())
        } else {
            format!("localfs://{}", parsed_uri.bucket.as_ref().unwrap())
        };

        let object_store = ObjectStore::new(&bucket_uri, config).unwrap();

        if let Some(callback) = callback {
            object_store
                .list_files_with_callback(
                    parsed_uri.path.as_deref(),
                    recursive,
                    max_files,
                    filter,
                    callback,
                )
                .await?;
            Ok(None)
        } else {
            let file_objects = object_store
                .list_files(
                    parsed_uri.path.as_deref(),
                    recursive,
                    max_files,
                    filter,
                )
                .await?;
            Ok(Some(ListObjectsResult::FileObjects(file_objects)))
        }
    }

    pub async fn execute_query(
        &self,
        statement: &str,
        config: &EnvironmentConfig,
        callback: Option<CallbackWrapper<FileObject>>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
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
        callback: Option<CallbackWrapper<FileObject>>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
        if let SetExpr::Select(select) = &*query.body {
            if select.projection.len() == 1
                && matches!(select.projection[0], SelectItem::Wildcard(_))
            {
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

                    let result = self
                        .list_objects(
                            &uri,
                            config,
                            true,
                            None,
                            &None,
                            callback.clone(),
                        )
                        .await;

                    match result {
                        Err(LakestreamError::NoBucketInUri(_)) => {
                            // Assume uri is a pointer to a database file (e.g. .sql, .parquet)
                            return self
                                .query_object(&uri, config, query, callback)
                                .await;
                        }
                        _ => return result,
                    }
                }
            }
        }

        Err(LakestreamError::InternalError(
            "Query does not match 'SELECT * FROM uri' pattern".to_string(),
        ))
    }

    async fn query_object(
        &self,
        _uri: &str,
        _config: &EnvironmentConfig,
        _query: &Query,
        _callback: Option<CallbackWrapper<FileObject>>,
    ) -> Result<Option<ListObjectsResult>, LakestreamError> {
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
        items: &mut RowItemVec,
    ) -> Result<(), LakestreamError>;
}
