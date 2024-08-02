use core::fmt;
use std::sync::Arc;

use log::error;

use crate::handlers::object_store::{ObjectStore, ObjectStoreBackend};
use crate::localfs::backend::LocalFsBackend;
use crate::s3::backend::S3Backend;
use crate::table::{StringColumn, TableRow};
use crate::{
    EnvironmentConfig, InternalError, Table, TableCallback, TableColumn,
    TableColumnValue,
};

pub struct ObjectStoreTable {
    columns: Vec<(String, Box<dyn TableColumn>)>, // Store columns in order
    callback: Option<Arc<dyn TableCallback>>,
}

impl ObjectStoreTable {
    pub fn new(selected_columns: &Option<Vec<&str>>) -> Self {
        let mut table = Self {
            columns: Vec::new(),
            callback: None,
        };

        // Considering future flexibility, even though we currently only expect "uri"
        let valid_columns = vec!["uri"]; // Only "uri" is valid for ObjectStoreTable

        if let Some(columns) = selected_columns {
            for &column in columns {
                if valid_columns.contains(&column) {
                    table
                        .add_column(column, Box::new(StringColumn(Vec::new())));
                } else {
                    panic!(
                        "Invalid column name for ObjectStoreTable: {}",
                        column
                    );
                }
            }
        } else {
            // If no columns are specified, add "uri" by default
            table.add_column("uri", Box::new(StringColumn(Vec::new())));
        }

        table
    }
}

impl Table for ObjectStoreTable {
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
        self.columns.push((name.to_string(), column_type));
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
            if let Some((_, column)) = self
                .columns
                .iter_mut()
                .find(|(name, _)| name == &column_name)
            {
                column.append(value)?;
            } else {
                return Err(InternalError::InternalError(format!(
                    "Column {} not found in table",
                    column_name
                )));
            }
        }

        Ok(())
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

impl ObjectStoreTable {
    pub async fn add_object_store(
        &mut self,
        object_store: ObjectStore,
    ) -> Result<(), InternalError> {
        let row_data = vec![(
            "uri".to_string(),
            TableColumnValue::StringColumn(object_store.uri()),
        )];
        self.add_row(row_data)
    }
}

fn print_row(row: &TableRow) {
    let uri = row
        .data()
        .iter()
        .find(|(key, _)| key == "uri")
        .map(|(_, value)| match value {
            TableColumnValue::StringColumn(val) => val,
            _ => "Invalid type", // Placeholder for error handling
        })
        .unwrap_or("URI not found");
    println!("{}", uri);
}

impl fmt::Debug for ObjectStoreTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_debug(f)
    }
}

pub async fn table_from_list_bucket(
    config: EnvironmentConfig,
    selected_columns: &Option<Vec<&str>>,
    max_files: Option<u32>,
    callback: Option<Arc<dyn TableCallback>>,
) -> Result<Box<dyn Table>, InternalError> {
    let uri = config.get("uri").unwrap_or(&"".to_string()).clone();

    let mut table = ObjectStoreTable::new(selected_columns);

    // if callback defined, set it
    if let Some(callback) = callback {
        table.set_callback(callback);
    }

    if uri.starts_with("s3://") {
        // Delegate the logic to the S3 backend
        S3Backend::list_buckets(config.clone(), max_files, &mut table).await?;
    } else if uri.starts_with("localfs://") {
        // Delegate the logic to the LocalFs backend
        LocalFsBackend::list_buckets(config.clone(), max_files, &mut table)
            .await?;
    } else {
        error!("Unsupported object store type: {}", uri);
    }
    Ok(Box::new(table))
}
