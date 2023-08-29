use std::collections::HashMap;
use std::sync::Arc;


#[derive(Debug)]
pub enum Response {
    Empty,
    Table(TableType),
    Binary {
        data: Arc<Vec<u8>>,
        metadata: Option<HashMap<String, String>>,
    },
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

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
}

type Row = Vec<Option<DataType>>;

#[derive(Debug)]
pub struct RowTable {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

#[derive(Debug)]
pub struct ColumnarData {
    pub data_type: DataType,
    pub data: Vec<Option<DataType>>,
}

#[derive(Debug)]
pub struct ColumnarTable {
    pub columns: HashMap<String, ColumnarData>,
}


#[derive(Debug)]
pub enum TableType {
    Row(Arc<RowTable>),
    Columnar(Arc<ColumnarTable>),
}

