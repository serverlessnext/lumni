use crate::{FileObject, RowItem};

pub enum ListObjectsResult {
    RowItems(Vec<RowItem>),
    FileObjects(Vec<FileObject>),
}
