use core::fmt;
use std::collections::HashMap;
use std::sync::Arc;

use crate::formatters::{bytes_human_readable, time_human_readable};
use crate::table::{
    OptionalUint64Column, StringColumn, TableRow, Uint64Column,
};
use crate::{FileObject, Table, TableCallback, TableColumn, TableColumnValue};

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
        table.add_column("name", Box::new(StringColumn(Vec::new())));
        table.add_column("size", Box::new(Uint64Column(Vec::new())));
        table
            .add_column("modified", Box::new(OptionalUint64Column(Vec::new())));
        // note: tags are not yet covered: Option<HashMap<String, String>>,

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
        row_data: HashMap<String, TableColumnValue>,
    ) -> Result<(), String> {
        if let Some(callback) = &self.callback {
            let mut row = TableRow::new(row_data.clone(), &print_row);
            callback.on_row_add(&mut row);
        }

        for (column_name, value) in row_data {
            if let Some(column) = self.columns.get_mut(&column_name) {
                column.append(value)?;
            } else {
                return Err(format!("Column not found: {}", column_name));
            }
        }

        Ok(())
    }

    fn print_items(&self) {
        let column_uri = self
            .columns
            .get("name")
            .expect("Column 'name' does not exist.");

        let string_column = column_uri
            .as_any()
            .downcast_ref::<StringColumn>()
            .expect("Column 'name' is not a StringColumn.");

        for value in string_column.values() {
            println!("{}", value);
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
            f.write_str(",\n")?;
        }
        f.write_str("}\n")
    }
}

fn print_row(row: &TableRow) {
    let full_path = true;
    let row_data = row.data();
    let name = row_data.get("name").expect("name not found").to_string();
    let fsize =
        extract_u64_value(row_data.get("size").expect("size not found"))
            .unwrap_or(0);
    let modified = extract_u64_value(
        row_data.get("modified").expect("modified not found"),
    );

    let name_without_trailing_slash = name.trim_end_matches('/');
    let mut name_to_print = if full_path {
        name_without_trailing_slash.to_string()
    } else {
        name_without_trailing_slash
            .split('/')
            .last()
            .unwrap_or(name_without_trailing_slash)
            .to_string()
    };

    if name.ends_with('/') {
        name_to_print.push('/');
    }

    println!(
        "{}",
        format!(
            "{:8} {} {}",
            bytes_human_readable(fsize),
            if let Some(mtime) = modified {
                time_human_readable(mtime)
            } else {
                "PRE".to_string()
            },
            name_to_print
        )
    );
}

impl FileObjectTable {
    pub async fn add_file_objects(
        &mut self,
        file_objects: Vec<FileObject>,
    ) -> Result<(), String> {
        for file_object in file_objects {
            let mut row_data = HashMap::new();
            row_data.insert(
                "name".to_string(),
                TableColumnValue::StringColumn(file_object.name().to_string()),
            );
            row_data.insert(
                "size".to_string(),
                TableColumnValue::Uint64Column(file_object.size()),
            );
            row_data.insert(
                "modified".to_string(),
                TableColumnValue::OptionalUint64Column(file_object.modified()),
            );
            self.add_row(row_data)?;
        }
        Ok(())
    }
}

impl fmt::Debug for FileObjectTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_debug(f)
    }
}

fn extract_u64_value(modified: &TableColumnValue) -> Option<u64> {
    match modified {
        TableColumnValue::Uint64Column(value) => Some(*value),
        TableColumnValue::OptionalUint64Column(Some(value)) => Some(*value),
        TableColumnValue::OptionalUint64Column(None) => None,
        // this should never happen, and if it does, it's a programming error
        _ => panic!(
            "Unexpected column type; expected Uint64Column or \
             OptionalUint64Column"
        ),
    }
}
