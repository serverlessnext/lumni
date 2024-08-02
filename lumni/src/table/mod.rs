pub mod columns;
pub mod file_object;
pub mod object_store;

use core::fmt;
use std::fmt::Debug;
use std::sync::Arc;

pub use columns::*;
pub use file_object::FileObjectTable;
pub use object_store::ObjectStoreTable;

use crate::InternalError;

pub struct TableRow<'a> {
    data: Vec<(String, TableColumnValue)>,
    print_fn: Option<&'a (dyn Fn(&TableRow) + 'a)>,
}

impl<'a> TableRow<'a> {
    pub fn new(
        data: Vec<(String, TableColumnValue)>,
        print_fn: Option<&'a (dyn Fn(&TableRow) + 'a)>,
    ) -> Self {
        Self { data, print_fn }
    }

    pub fn data(&self) -> &[(String, TableColumnValue)] {
        &self.data
    }

    pub fn print(&self) {
        if let Some(print_fn) = self.print_fn {
            print_fn(self); // custom print function
        } else {
            self.print_columns(); // default print function
        }
    }

    pub fn print_columns(&self) {
        let values_to_print: Vec<String> = self
            .data
            .iter()
            .map(|(_, value)| {
                let value_str = match value {
                    TableColumnValue::Int32Column(val) => val.to_string(),
                    TableColumnValue::Uint64Column(val) => val.to_string(),
                    TableColumnValue::FloatColumn(val) => val.to_string(),
                    TableColumnValue::StringColumn(val) => val.clone(),
                    TableColumnValue::OptionalInt32Column(Some(val)) => {
                        val.to_string()
                    }
                    TableColumnValue::OptionalUint64Column(Some(val)) => {
                        val.to_string()
                    }
                    TableColumnValue::OptionalFloatColumn(Some(val)) => {
                        val.to_string()
                    }
                    TableColumnValue::OptionalStringColumn(Some(val)) => {
                        val.clone()
                    }
                    _ => "None".to_string(), // Handle None cases for Optional values
                };
                format!("{}", value_str)
            })
            .collect();

        println!("{}", values_to_print.join(","));
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

pub trait Table: Debug {
    fn len(&self) -> usize;
    fn add_column(&mut self, name: &str, column_type: Box<dyn TableColumn>);
    fn set_callback(&mut self, callback: Arc<dyn TableCallback>);
    fn add_row(
        &mut self,
        row_data: Vec<(String, TableColumnValue)>,
    ) -> Result<(), InternalError>;
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}
