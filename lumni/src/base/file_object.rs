use std::collections::HashMap;

use crate::table::TableColumnValue;

#[derive(Debug, Clone)]
pub struct FileObject {
    name: String,
    size: u64,
    r#type: FileType,
    // modified time is i64 to align with db, also allows pre-epoch timestamps
    modified: Option<i64>,
    tags: Option<HashMap<String, String>>,
}

impl FileObject {
    pub fn new(
        name: String,
        size: u64,
        r#type: FileType,
        modified: Option<i64>,
        tags: Option<HashMap<String, String>>,
    ) -> Self {
        FileObject {
            name,
            size,
            r#type,
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

    pub fn is_directory(&self) -> bool {
        self.r#type == FileType::Directory
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
            "type" => Some(TableColumnValue::Uint8Column(self.r#type.to_u8())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileType {
    Directory = 0,
    RegularFile = 1,
    SymbolicLink = 2,
    BlockDevice = 3,
    CharDevice = 4,
    Fifo = 5,
    Socket = 6,
    Unknown = 255,
}

impl FileType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => FileType::Directory,
            1 => FileType::RegularFile,
            2 => FileType::SymbolicLink,
            3 => FileType::BlockDevice,
            4 => FileType::CharDevice,
            5 => FileType::Fifo,
            6 => FileType::Socket,
            _ => FileType::Unknown,
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}
