use core::{fmt, panic};
use std::collections::HashMap;
use std::sync::Arc;

use log::error;

use crate::api::object_store_handler::ObjectStoreBackend;
use crate::localfs::backend::LocalFsBackend;
use crate::s3::backend::S3Backend;
use crate::table::StringColumn;
use crate::{
    EnvironmentConfig, LakestreamError,
    ObjectStore,
    Table, TableCallback, TableColumn, TableColumnValue,
};


pub struct ObjectStoreTable {
    columns: HashMap<String, Box<dyn TableColumn>>,
    callback: Option<Arc<dyn TableCallback>>,
}

impl ObjectStoreTable {
    pub fn new() -> Self {
        let mut table = Self {
            columns: HashMap::new(),
            callback: None,
        };

        table.add_column("uri", Box::new(StringColumn(Vec::new())));
        table
    }
}

impl Table for ObjectStoreTable {
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
        if let Some(column_uri) = self.columns.get("uri") {
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

impl ObjectStoreTable {
    pub async fn add_object_store(
        &mut self,
        object_store: ObjectStore,
    ) -> Result<(), String> {
        let mut row = HashMap::new();
        row.insert(
            "uri".to_string(),
            TableColumnValue::StringColumn(object_store.uri()),
        );
        self.add_row(row)
    }
}

impl fmt::Debug for ObjectStoreTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_debug(f)
    }
}

pub async fn table_from_list_bucket(
    config: EnvironmentConfig,
    callback: Option<Arc<dyn TableCallback>>,
) -> Result<Box<dyn Table>, LakestreamError> {
    let uri = config.get("uri").unwrap_or(&"".to_string()).clone();

    let mut table = ObjectStoreTable::new();

    // if callback defined, set it
    if let Some(callback) = callback {
        table.set_callback(callback);
    }

    if uri.starts_with("s3://") {
        // Delegate the logic to the S3 backend
        S3Backend::list_buckets(config.clone(), &mut table).await?;
    } else if uri.starts_with("localfs://") {
        // Delegate the logic to the LocalFs backend
        LocalFsBackend::list_buckets(config.clone(), &mut table).await?;
    } else {
        error!("Unsupported object store type: {}", uri);
    }
    Ok(Box::new(table))
}

