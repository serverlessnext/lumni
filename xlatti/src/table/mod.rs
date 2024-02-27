pub mod columns;
pub mod file_object;
pub mod object_store;

use core::fmt;
use std::sync::Arc;

pub use columns::*;
pub use file_object::FileObjectTable;
pub use object_store::ObjectStoreTable;

pub struct TableRow<'a> {
    data: Vec<(String, TableColumnValue)>,
    print_fn: &'a (dyn Fn(&TableRow) + 'a),
}

impl<'a> TableRow<'a> {
    pub fn new(
        data: Vec<(String, TableColumnValue)>,
        print_fn: &'a (dyn Fn(&TableRow) + 'a),
    ) -> Self {
        Self { data, print_fn }
    }

    pub fn data(&self) -> &[(String, TableColumnValue)] {
        &self.data
    }

    pub fn print(&self) {
        (self.print_fn)(self);
    }
}

impl<'a> Clone for TableRow<'a> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            print_fn: self.print_fn,
        }
    }
}
pub trait TableCallback: Send + Sync {
    fn on_row_add(&self, row: &mut TableRow);
}

pub trait Table {
    fn len(&self) -> usize;
    fn add_column(&mut self, name: &str, column_type: Box<dyn TableColumn>);
    fn set_callback(&mut self, callback: Arc<dyn TableCallback>);
    fn add_row(
        &mut self,
        row_data: Vec<(String, TableColumnValue)>,
    ) -> Result<(), String>;
    fn print_items(&self);
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}
