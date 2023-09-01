use std::collections::HashMap;
use std::fmt::Debug;

use super::DataType;

pub enum TableType {
    Row,
    Columnar,
}

pub trait Table: Debug + Send + Sync {
    fn get_row_count(&self) -> usize;
    fn table_type(&self) -> TableType;
    fn as_row(&self) -> Option<&RowTable>;
    fn as_columnar(&self) -> Option<&ColumnarTable>;
}

#[derive(Debug)]
pub struct RowTable {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
}

pub type Row = Vec<Option<DataType>>;

impl Table for RowTable {
    fn get_row_count(&self) -> usize {
        self.rows.len()
    }
    fn table_type(&self) -> TableType {
        TableType::Row
    }
    fn as_row(&self) -> Option<&RowTable> {
        Some(self)
    }
    fn as_columnar(&self) -> Option<&ColumnarTable> {
        None
    }
}

#[derive(Debug)]
pub struct ColumnarTable {
    pub columns: HashMap<String, ColumnarData>,
}

#[derive(Debug)]
pub struct ColumnarData {
    pub data_type: DataType,
    pub data: Vec<Option<DataType>>,
}

impl Table for ColumnarTable {
    fn get_row_count(&self) -> usize {
        if let Some(first_column) = self.columns.values().next() {
            first_column.data.len()
        } else {
            0
        }
    }
    fn table_type(&self) -> TableType {
        TableType::Columnar
    }
    fn as_row(&self) -> Option<&RowTable> {
        None
    }
    fn as_columnar(&self) -> Option<&ColumnarTable> {
        Some(self)
    }
}
