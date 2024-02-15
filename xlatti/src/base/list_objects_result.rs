use crate::RowItem;

pub enum ListObjectsResult {
    RowItems(Vec<RowItem>),
    FileObjects(Vec<RowItem>),
}
