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

//struct FileObjectTable {
//    columns: HashMap<String, Box<dyn TableColumn>>,
//    callback: Option<Arc<dyn TableCallback>>,
//}
//
//impl FileObjectTable {
//    pub fn new() -> Self {
//        let mut table = Self {
//            columns: HashMap::new(),
//            callback: None,
//        };
//
//        table.add_column("name", Box::new(StringColumn(Vec::new())));
//        table.add_column("size", Box::new(IntColumn(Vec::new())));
//        table.add_column("ctime", Box::new(FloatColumn(Vec::new())));
//        table
//    }
//}

//impl Table for FileObjectTable {
//    fn add_column(&mut self, name: &str, column_type: Box<dyn TableColumn>) {
//        self.columns.insert(name.to_string(), column_type);
//    }
//
//    fn set_callback(&mut self, callback: Arc<dyn TableCallback>) {
//        self.callback = Some(callback);
//    }
//
//    fn add_row(
//        &mut self,
//        row: HashMap<String, TableColumnValue>,
//    ) -> Result<(), String> {
//        if let Some(callback) = &self.callback {
//            let mut row_for_callback = row.clone(); 
//            callback.on_row_add(&mut row_for_callback);
//        }
//
//        // Add row values to their respective columns
//        for (column_name, value) in row {
//            if let Some(column) = self.columns.get_mut(&column_name) {
//                column.append(value)?;
//            } else {
//                return Err(format!("Column not found: {}", column_name));
//            }
//        }
//
//        Ok(())
//    }
//
//    fn print_items(&self) {
//        println!("Printing items");
//    }
//
//    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//        f.debug_struct("Table")
//            .field("callback", &"Callback Omitted")
//            .finish()?;
//
//        f.write_str("columns: {\n")?;
//        for (name, column) in &self.columns {
//            write!(f, "    {}: ", name)?;
//            column.fmt_debug(f)?;
//            f.write_str(",\n")?;
//        }
//        f.write_str("}\n")
//    }
//}

//impl FileObjectTable {
//    // Add a specific method for adding a FileObject row
//    pub async fn add_file_object(
//        &mut self,
//        name: String,
//        size: i32,
//        ctime: f64,
//    ) -> Result<(), String> {
//        let mut row = HashMap::new();
//        row.insert("name".to_string(), TableColumnValue::String(name));
//        row.insert("size".to_string(), TableColumnValue::Int(size));
//        row.insert("ctime".to_string(), TableColumnValue::Float(ctime));
//        self.add_row(row)
//    }
//}
//
//impl fmt::Debug for FileObjectTable {
//    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//        self.fmt_debug(f)
//    }
//}

//pub async fn table_from_list(
//    config: EnvironmentConfig,
//    callback: &Option<AsyncRowCallback>,
//) -> Result<Box<dyn Table>, LakestreamError> {
//    let uri = config.get("uri").unwrap_or(&"".to_string()).clone();
//
//    let mut file_table = FileObjectTable::new();
//    // set callback
//    file_table.set_callback(Arc::new(FileObjectCallback));
//    file_table
//        .add_file_object("example_file.txt".to_string(), 1024, 1_607_841_200.0)
//        .await?;
//    // Debug print to verify the table's contents
//
//    Ok(Box::new(file_table))
//}

//struct FileObjectCallback;
//
//impl TableCallback for FileObjectCallback {
//    fn on_row_add(&self, row: &mut HashMap<String, TableColumnValue>) {
//        println!("FileObject added:");
//        if let Some(TableColumnValue::Float(_)) = row.get("ctime") {
//            row.insert("ctime".to_string(), TableColumnValue::Float(100.0));
//        }
//        for (column_name, value) in row {
//            println!("Column: {}", column_name);
//            println!("{}: {:?}", column_name, value);
//        }
//    }
//}
