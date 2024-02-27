use core::{fmt, panic};
use std::collections::HashMap;
use std::sync::Arc;

use crate::formatters::{bytes_human_readable, time_human_readable};
use crate::table::{
    OptionalUint64Column, StringColumn, TableRow, Uint64Column,
};
use crate::{FileObject, Table, TableCallback, TableColumn, TableColumnValue};

pub struct FileObjectTable {
    columns: Vec<(String, Box<dyn TableColumn>)>, // Store columns in order
    column_index: HashMap<String, usize>,         // store order of columns
    callback: Option<Arc<dyn TableCallback>>,
    print_function: fn(&TableRow),
}

impl FileObjectTable {
    pub fn new(selected_columns: &Option<Vec<&str>>) -> Self {
        let print_function: fn(&TableRow) = if selected_columns.is_some() {
            print_selected
        } else {
            print_row
        };

        let mut table = Self {
            columns: Vec::new(),
            column_index: HashMap::new(),
            callback: None,
            print_function,
        };

        // Define a list of valid column names
        let valid_columns = vec!["name", "size", "modified"];

        if let Some(columns) = selected_columns {
            for &column in columns {
                match column {
                    "name" => table
                        .add_column("name", Box::new(StringColumn(Vec::new()))),
                    "size" => table
                        .add_column("size", Box::new(Uint64Column(Vec::new()))),
                    "modified" => table.add_column(
                        "modified",
                        Box::new(OptionalUint64Column(Vec::new())),
                    ),
                    _ => panic!("Invalid column name: {}", column),
                }
            }
        } else {
            // If no selected_columns provided, add all valid columns by default
            for &column in &valid_columns {
                match column {
                    "name" => table
                        .add_column("name", Box::new(StringColumn(Vec::new()))),
                    "size" => table
                        .add_column("size", Box::new(Uint64Column(Vec::new()))),
                    "modified" => table.add_column(
                        "modified",
                        Box::new(OptionalUint64Column(Vec::new())),
                    ),
                    // should not be reachable because valid_columns is hardcoded
                    _ => panic!("Invalid column name: {}", column),
                }
            }
        }
        table
    }
}

impl Table for FileObjectTable {
    fn len(&self) -> usize {
        if self.columns.is_empty() {
            0
        } else {
            // Since all columns should have the same length,
            // return the length of the first column's data.
            self.columns[0].1.len()
        }
    }

    fn add_column(&mut self, name: &str, column_type: Box<dyn TableColumn>) {
        let index = self.columns.len();
        self.columns.push((name.to_string(), column_type));
        self.column_index.insert(name.to_string(), index);
    }

    fn set_callback(&mut self, callback: Arc<dyn TableCallback>) {
        self.callback = Some(callback);
    }

    fn add_row(
        &mut self,
        row_data: Vec<(String, TableColumnValue)>,
    ) -> Result<(), String> {
        if let Some(callback) = &self.callback {
            let mut row = TableRow::new(row_data.clone(), &self.print_function);
            callback.on_row_add(&mut row);
        }
        for (column_name, value) in row_data {
            if let Some(&index) = self.column_index.get(&column_name) {
                let (_, column) = &mut self.columns[index];
                column.append(value)?;
            } else {
                return Err(format!("Column '{}' not found", column_name));
            }
        }

        Ok(())
    }

    fn print_items(&self) {
        let column = self.columns.iter().find(|(name, _)| name == "name");

        if let Some((_, column)) = column {
            // Attempt to downcast to a StringColumn
            if let Some(string_column) =
                column.as_any().downcast_ref::<StringColumn>()
            {
                // Iterate over the values in the StringColumn and print them
                for value in string_column.values() {
                    println!("{}", value);
                }
            } else {
                println!("Column 'name' is not a StringColumn.");
            }
        } else {
            println!("Column 'name' does not exist.");
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

    let name = row_data
        .iter()
        .find(|(key, _)| key == "name")
        .map(|(_, value)| match value {
            TableColumnValue::StringColumn(val)
            | TableColumnValue::OptionalStringColumn(Some(val)) => val,
            _ => panic!("name column not found or not a string"),
        })
        .expect("name not found")
        .to_string();

    let fsize = row_data
        .iter()
        .find(|(key, _)| key == "size")
        .map(|(_, value)| extract_u64_value(value))
        .flatten()
        .unwrap_or(0); // defaults to 0 if None

    let modified = row_data
        .iter()
        .find(|(key, _)| key == "modified")
        .map(|(_, value)| extract_u64_value(value))
        .flatten();

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

fn print_selected(row: &TableRow) {
    let row_data = row.data();
    let mut values_to_print = Vec::new();

    // Iterate over all columns present in the row
    for (_key, value) in row_data {
        // Prepare the value for printing, adjust as necessary
        let value_str = match value {
            TableColumnValue::Int32Column(val) => val.to_string(),
            TableColumnValue::Uint64Column(val) => val.to_string(),
            TableColumnValue::FloatColumn(val) => val.to_string(),
            TableColumnValue::StringColumn(val) => val.clone(),
            TableColumnValue::OptionalInt32Column(Some(val)) => val.to_string(),
            TableColumnValue::OptionalUint64Column(Some(val)) => {
                val.to_string()
            }
            TableColumnValue::OptionalFloatColumn(Some(val)) => val.to_string(),
            TableColumnValue::OptionalStringColumn(Some(val)) => val.clone(),
            TableColumnValue::OptionalInt32Column(None) => "None".to_string(),
            TableColumnValue::OptionalUint64Column(None) => "None".to_string(),
            TableColumnValue::OptionalFloatColumn(None) => "None".to_string(),
            TableColumnValue::OptionalStringColumn(None) => "None".to_string(),
        };
        values_to_print.push(value_str);
    }

    // Join the values with commas and print
    println!("{}", values_to_print.join(","));
}

impl FileObjectTable {
    pub async fn add_file_objects(
        &mut self,
        file_objects: Vec<FileObject>,
    ) -> Result<(), String> {
        for file_object in file_objects {
            let mut row_data: Vec<(String, TableColumnValue)> = Vec::new();

            for (column_name, _) in &self.columns {
                if let Some(value) =
                    file_object.get_value_by_column_name(column_name)
                {
                    row_data.push((column_name.clone(), value));
                } else {
                    panic!("Column not found: {}", column_name);
                }
            }

            // Now add the row_data for this file_object to the table
            self.add_row(row_data)?;
        }
        Ok(())
    }

    pub async fn add_rows(
        &mut self,
        rows: Vec<HashMap<String, TableColumnValue>>,
    ) -> Result<(), String> {
        for row_data in rows {
            let mut row_vec: Vec<(String, TableColumnValue)> = Vec::new();

            // Iterate over self.columns to maintain the defined order
            for (column_name, _) in &self.columns {
                if let Some(value) = row_data.get(column_name) {
                    row_vec.push((column_name.clone(), value.clone()));
                } else {
                    panic!("Column not found: {}", column_name);
                }
            }
            self.add_row(row_vec)?;
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
