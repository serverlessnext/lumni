pub mod object_store;
pub mod file_object;
pub mod columns;

use core::fmt;
use std::collections::HashMap;
use std::sync::Arc;

pub use object_store::ObjectStoreTable;
pub use file_object::FileObjectTable;

pub use columns::{
    TableColumn, TableColumnValue,
    Uint64Column, StringColumn,
};

pub trait TableCallback: Send + Sync {
    fn on_row_add(&self, row: &mut HashMap<String, TableColumnValue>);
}

pub trait Table {
    fn len(&self) -> usize;
    fn add_column(&mut self, name: &str, column_type: Box<dyn TableColumn>);
    fn set_callback(&mut self, callback: Arc<dyn TableCallback>);
    fn add_row(
        &mut self,
        row: HashMap<String, TableColumnValue>,
    ) -> Result<(), String>;
    fn print_items(&self);
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}