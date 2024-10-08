use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;

use super::get::get_object;
use super::head::head_object;
use super::list::list_files;
use crate::base::config::EnvironmentConfig;
use crate::handlers::object_store::ObjectStoreTrait;
use crate::table::FileObjectTable;
use crate::{FileObjectFilter, InternalError};

#[derive(Debug, Clone)]
pub struct LocalFsBucket {
    name: String,
    #[allow(dead_code)]
    config: EnvironmentConfig,
}

impl LocalFsBucket {
    pub fn new(
        name: &str,
        config: EnvironmentConfig,
    ) -> Result<LocalFsBucket, &'static str> {
        Ok(LocalFsBucket {
            name: name.to_string(),
            config,
        })
    }
}

#[async_trait(?Send)]
impl ObjectStoreTrait for LocalFsBucket {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &EnvironmentConfig {
        &self.config
    }

    async fn list_files(
        &self,
        prefix: Option<&str>,
        selected_columns: &Option<Vec<&str>>,
        skip_hidden: bool,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
        table: &mut FileObjectTable,
    ) -> Result<(), InternalError> {
        let path = match prefix {
            Some(prefix) => Path::new(&self.name).join(prefix),
            None => Path::new(&self.name).to_path_buf(),
        };

        // to be considered a Bucket, path must be a directory
        if !path.is_dir() {
            return Err(InternalError::NoBucketInUri(
                path.to_string_lossy().to_string(),
            ));
        }
        list_files(
            &path,
            selected_columns,
            max_keys,
            skip_hidden,
            recursive,
            filter,
            table,
        )
        .await?;
        Ok(())
    }

    async fn get_object(
        &self,
        key: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), InternalError> {
        let path = Path::new(&self.name);
        get_object(path, key, data).await
    }

    async fn head_object(
        &self,
        _key: &str,
    ) -> Result<(u16, HashMap<String, String>), InternalError> {
        let path = Path::new(&self.name);
        head_object(path, _key).await?;
        Ok((200, HashMap::new()))
    }
}
