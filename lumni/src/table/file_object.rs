use core::{fmt, panic};
use std::collections::HashMap;
use std::sync::Arc;

use crate::table::{
    OptionalInt64Column, StringColumn, TableRow, Uint64Column, Uint8Column,
};
use crate::utils::formatters::{bytes_human_readable, time_human_readable};
use crate::{
    FileObject, InternalError, Table, TableCallback, TableColumn,
    TableColumnValue,
};

pub struct FileObjectTable {
    columns: Vec<(String, Box<dyn TableColumn>)>, // Store columns in order
    column_index: HashMap<String, usize>,         // store order of columns
    callback: Option<Arc<dyn TableCallback>>,
}

impl FileObjectTable {
    pub fn new(
        selected_columns: &Option<Vec<&str>>,
        callback: Option<Arc<dyn TableCallback>>,
    ) -> Self {
        let mut table = Self {
            columns: Vec::new(),
            column_index: HashMap::new(),
            callback,
        };

        // Define a list of valid column names
        let valid_columns = vec!["name", "size", "modified", "type"];

        if let Some(columns) = selected_columns {
            for &column in columns {
                match column {
                    "name" => table
                        .add_column("name", Box::new(StringColumn(Vec::new()))),
                    "size" => table
                        .add_column("size", Box::new(Uint64Column(Vec::new()))),
                    "modified" => table.add_column(
                        "modified",
                        Box::new(OptionalInt64Column(Vec::new())),
                    ),
                    "type" => {
                        table.add_column(
                            "type",
                            Box::new(Uint8Column(Vec::new())),
                        );
                    }
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
                        Box::new(OptionalInt64Column(Vec::new())),
                    ),
                    "type" => {
                        table.add_column(
                            "type",
                            Box::new(Uint8Column(Vec::new())),
                        );
                    }
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
    ) -> Result<(), InternalError> {
        if let Some(callback) = &self.callback {
            let mut row = TableRow::new(row_data.clone(), Some(&print_row));
            callback.on_row_add(&mut row);
        }

        for (column_name, value) in row_data {
            if let Some(&index) = self.column_index.get(&column_name) {
                let (_, column) = &mut self.columns[index];
                column.append(value)?;
            } else {
                return Err(InternalError::InternalError(format!(
                    "Column '{}' not found",
                    column_name
                )));
            }
        }
        Ok(())
    }

    fn get_row(&self, index: usize) -> Option<TableRow> {
        // TODO: should implement a more efficient way to get the correct type property
        // should be fixed for FileObjectTable
        if index >= self.len() {
            return None;
        }
        let row_data: Vec<(String, TableColumnValue)> = self
            .columns
            .iter()
            .filter_map(|(name, column)| {
                let value = match column.as_any().downcast_ref::<StringColumn>()
                {
                    Some(c) => {
                        c.0.get(index)
                            .map(|s| TableColumnValue::StringColumn(s.clone()))
                    }
                    None => {
                        match column.as_any().downcast_ref::<Uint64Column>() {
                            Some(c) => {
                                c.0.get(index)
                                    .map(|&u| TableColumnValue::Uint64Column(u))
                            }
                            None => match column
                                .as_any()
                                .downcast_ref::<OptionalInt64Column>()
                            {
                                Some(c) => c.0.get(index).map(|&oi| {
                                    TableColumnValue::OptionalInt64Column(oi)
                                }),
                                None => {
                                    match column
                                        .as_any()
                                        .downcast_ref::<Uint8Column>()
                                    {
                                        Some(c) => c.0.get(index).map(|&u| {
                                            TableColumnValue::Uint8Column(u)
                                        }),
                                        None => None,
                                    }
                                }
                            },
                        }
                    }
                };
                value.map(|v| (name.clone(), v))
            })
            .collect();

        if row_data.is_empty() {
            None
        } else {
            Some(TableRow::new(row_data, Some(&print_row)))
        }
    }

    fn get_value(
        &self,
        index: usize,
        column_name: &str,
    ) -> Option<TableColumnValue> {
        if index >= self.len() {
            return None;
        }

        self.column_index.get(column_name).and_then(|&col_index| {
            let (_, column) = &self.columns[col_index];
            match column.as_any().downcast_ref::<StringColumn>() {
                Some(c) => {
                    c.0.get(index)
                        .map(|s| TableColumnValue::StringColumn(s.clone()))
                }
                None => match column.as_any().downcast_ref::<Uint64Column>() {
                    Some(c) => {
                        c.0.get(index)
                            .map(|&u| TableColumnValue::Uint64Column(u))
                    }
                    None => match column
                        .as_any()
                        .downcast_ref::<OptionalInt64Column>()
                    {
                        Some(c) => c.0.get(index).map(|&oi| {
                            TableColumnValue::OptionalInt64Column(oi)
                        }),
                        None => None,
                    },
                },
            }
        })
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
        .map(|(_, value)| extract_i64_value(value))
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

impl FileObjectTable {
    pub async fn add_file_objects(
        &mut self,
        file_objects: Vec<FileObject>,
    ) -> Result<(), InternalError> {
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
    ) -> Result<(), InternalError> {
        for row_data in rows.iter() {
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

fn extract_i64_value(modified: &TableColumnValue) -> Option<i64> {
    match modified {
        TableColumnValue::Int64Column(value) => Some(*value),
        TableColumnValue::OptionalInt64Column(Some(value)) => Some(*value),
        TableColumnValue::OptionalInt64Column(None) => None,
        // this should never happen, and if it does, it's a programming error
        _ => panic!(
            "Unexpected column type; expected Int64Column or \
             OptionalInt64Column"
        ),
    }
}
