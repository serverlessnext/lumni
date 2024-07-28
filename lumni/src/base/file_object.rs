use std::collections::HashMap;

use crate::table::TableColumnValue;

#[derive(Debug, Clone)]
pub struct FileObject {
    name: String,
    size: u64,
    // modified time is i64 to align with db, also allows pre-epoch timestamps
    modified: Option<i64>,
    tags: Option<HashMap<String, String>>,
}

impl FileObject {
    pub fn new(
        name: String,
        size: u64,
        modified: Option<i64>,
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

    pub fn modified(&self) -> Option<i64> {
        self.modified
    }

    pub fn tags(&self) -> &Option<HashMap<String, String>> {
        &self.tags
    }

    pub fn get_value_by_column_name(
        &self,
        column_name: &str,
    ) -> Option<TableColumnValue> {
        match column_name {
            "name" => Some(TableColumnValue::StringColumn(self.name.clone())),
            "size" => Some(TableColumnValue::Uint64Column(self.size)),
            "modified" => self
                .modified
                .map(|val| TableColumnValue::OptionalInt64Column(Some(val))),
            _ => None,
        }
    }
}
