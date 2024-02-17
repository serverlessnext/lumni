use std::collections::HashMap;

use crate::base::callback_wrapper::CallbackItem;
use crate::utils::formatters::{bytes_human_readable, time_human_readable};

#[derive(Debug, Clone)]
pub struct FileObject {
    name: String,
    size: u64,
    modified: Option<u64>,
    tags: Option<HashMap<String, String>>,
}

impl FileObject {
    pub fn new(
        name: String,
        size: u64,
        modified: Option<u64>,
        tags: Option<HashMap<String, String>>,
    ) -> Self {
        FileObject {
            name,
            size,
            modified,
            tags,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn modified(&self) -> Option<u64> {
        self.modified
    }

    pub fn tags(&self) -> &Option<HashMap<String, String>> {
        &self.tags
    }

    pub fn println_path(&self) -> String {
        // TODO: move to table.file_object
        let full_path = true;
        let name = self.name();
        let fsize = self.size();
        let modified = self.modified();

        let name_without_trailing_slash = name.trim_end_matches('/');
        let mut name_to_print = if full_path {
            name_without_trailing_slash.to_string()
        } else {
            name_without_trailing_slash
                .split('/')
                .last()
                .unwrap_or(name_without_trailing_slash)
                .to_string()
        };

        if self.name.ends_with('/') {
            name_to_print.push('/');
        }

        format!(
            "{:8} {} {}",
            bytes_human_readable(fsize),
            if let Some(mtime) = modified {
                time_human_readable(mtime)
            } else {
                "PRE".to_string()
            },
            name_to_print
        )
    }
}
