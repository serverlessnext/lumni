
use std::collections::HashMap;
use std::sync::Arc;

use super::Table;
use super::AnyKeyValue;

#[derive(Debug)]
pub enum Data {
    Empty,
    KeyValue(Arc<dyn AnyKeyValue>),
    Table(Arc<dyn Table>),
    Binary {
        data: Arc<Vec<u8>>,
        metadata: Option<HashMap<String, String>>,
    },
    // TODO: Document(DocumentType),
}

#[derive(Debug)]
pub enum DataType {
    Boolean(bool),
    String(String),
    Integer32(i32),
    Integer64(i64),
    Float64(f64),
    BinaryData(Vec<u8>),
    Array(Vec<DataType>),
}
