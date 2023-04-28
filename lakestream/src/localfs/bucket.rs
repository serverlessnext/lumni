use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;

use super::list::list_files;
use crate::base::interfaces::ObjectStoreTrait;
use crate::{FileObject, FileObjectFilter, LakestreamError};

pub struct LocalFs {
    name: String,
    #[allow(dead_code)]
    config: HashMap<String, String>,
}

impl LocalFs {
    pub fn new(
        name: &str,
        config: HashMap<String, String>,
    ) -> Result<LocalFs, &'static str> {
        Ok(LocalFs {
            name: name.to_string(),
            config,
        })
    }
}

#[async_trait(?Send)]
impl ObjectStoreTrait for LocalFs {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &HashMap<String, String> {
        &self.config
    }

    async fn list_files(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_keys: Option<u32>,
        filter: &Option<FileObjectFilter>,
    ) -> Result<Vec<FileObject>, LakestreamError> {
        let path = match prefix {
            Some(prefix) => Path::new(&self.name).join(prefix),
            None => Path::new(&self.name).to_path_buf(),
        };
        Ok(list_files(&path, max_keys, recursive, filter))
    }
}
