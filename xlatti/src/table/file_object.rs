use core::{fmt, panic};
use std::collections::HashMap;
use std::sync::Arc;

use crate::table::{
    Uint64Column, StringColumn,
};
use crate::{
    FileObject,
    Table, TableCallback, TableColumn, TableColumnValue,
};


pub struct FileObjectTable {
    columns: HashMap<String, Box<dyn TableColumn>>,
    callback: Option<Arc<dyn TableCallback>>,
}

impl FileObjectTable {
    pub fn new() -> Self {
        let mut table = Self {
            columns: HashMap::new(),
            callback: None,
        };
        //name: String,
        //size: u64,
        
        // TODO: add ability to have optional value columns
        //modified: Option<u64>,
        //tags: Option<HashMap<String, String>>,

        table.add_column("name", Box::new(StringColumn(Vec::new())));
        table.add_column("size", Box::new(Uint64Column(Vec::new())));

        table
    }
}

impl Table for FileObjectTable {
    fn len(&self) -> usize {
        if self.columns.is_empty() {
            0
        } else {
            // Return the length of the first column found
            self.columns.values().next().unwrap().len()
        }
    }
    fn add_column(&mut self, name: &str, column_type: Box<dyn TableColumn>) {
        self.columns.insert(name.to_string(), column_type);
    }

    fn set_callback(&mut self, callback: Arc<dyn TableCallback>) {
        self.callback = Some(callback);
    }

    fn add_row(
        &mut self,
        row: HashMap<String, TableColumnValue>,
    ) -> Result<(), String> {
        if let Some(callback) = &self.callback {
            let mut row_for_callback = row.clone();
            callback.on_row_add(&mut row_for_callback);
        }

        for (column_name, value) in row {
            if let Some(column) = self.columns.get_mut(&column_name) {
                column.append(value)?;
            } else {
                return Err(format!("Column not found: {}", column_name));
            }
        }

        Ok(())
    }

    fn print_items(&self) {
        if let Some(column_uri) = self.columns.get("name") {
            if let Some(string_column) =
                column_uri.as_any().downcast_ref::<StringColumn>()
            {
                for value in string_column.values() {
                    println!("{}", value);
                }
            } else {
                // This should never happen, if it does, it's a programming error.
                panic!("Column 'uri' is not a StringColumn or does not exist.");
            }
        }
    }

    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Table")
            .field("callback", &"Callback Omitted")
            .finish()?;

        f.write_str("columns: {\n")?;
        for (name, column) in &self.columns {
            write!(f, "    {}: ", name)?;
            write!(f, "{:?}", column)?;
            //column.fmt_debug(f)?;
            f.write_str(",\n")?;
        }
        f.write_str("}\n")
    }
}

impl FileObjectTable {
    pub async fn add_file_objects(
        &mut self,
        file_objects: Vec<FileObject>,
    ) -> Result<(), String> {
        for file_object in file_objects {
            let mut row = HashMap::new();
            row.insert(
                "name".to_string(),
                TableColumnValue::StringColumn(file_object.name().to_string()),
            );
            row.insert(
                "size".to_string(),
                TableColumnValue::Uint64Column(file_object.size()),
            );
            self.add_row(row)?;
        }
        Ok(())
    }
}
impl fmt::Debug for FileObjectTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_debug(f)
    }
}
