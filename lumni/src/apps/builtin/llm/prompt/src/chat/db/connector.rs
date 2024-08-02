use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Error as SqliteError, Transaction};

pub struct DatabaseConnector {
    connection: rusqlite::Connection,
    operation_queue: Arc<Mutex<VecDeque<String>>>,
}

impl DatabaseConnector {
    const SCHEMA_SQL: &'static str = include_str!("schema.sql");
    const EXPECTED_VERSION: &'static str = "1";
    const EXPECTED_IDENTIFIER: &'static str = "prompt.chat";

    pub fn new(sqlite_file: &PathBuf) -> Result<Self, SqliteError> {
        let connection = rusqlite::Connection::open(sqlite_file)?;
        let operation_queue = Arc::new(Mutex::new(VecDeque::new()));

        let mut conn = DatabaseConnector {
            connection,
            operation_queue,
        };
        conn.initialize_schema()?;
        Ok(conn)
    }

    fn initialize_schema(&mut self) -> Result<(), SqliteError> {
        let transaction = self.connection.transaction()?;

        // Check if the metadata table exists and has the correct version and identifier
        let (version, identifier, need_initialization) = {
            let mut stmt = transaction.prepare(
                "SELECT key, value FROM metadata WHERE key IN \
                 ('schema_version', 'schema_identifier')",
            );

            match stmt {
                Ok(ref mut stmt) => {
                    let result: Result<Vec<(String, String)>, SqliteError> =
                        stmt.query_map([], |row| {
                            Ok((row.get(0)?, row.get(1)?))
                        })?
                        .collect();

                    let mut version = None;
                    let mut identifier = None;

                    match result {
                        Ok(rows) if !rows.is_empty() => {
                            for (key, value) in rows {
                                match key.as_str() {
                                    "schema_version" => version = Some(value),
                                    "schema_identifier" => {
                                        identifier = Some(value)
                                    }
                                    _ => {}
                                }
                            }
                            log::debug!(
                                "Version: {:?}, Identifier: {:?}",
                                version,
                                identifier
                            );
                            (version, identifier, false)
                        }
                        Ok(_) | Err(SqliteError::QueryReturnedNoRows) => {
                            log::info!(
                                "No metadata found. Need initialization."
                            );
                            (None, None, true)
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(e) => match e {
                    SqliteError::SqliteFailure(_, Some(ref error_string))
                        if error_string.contains("no such table") =>
                    {
                        log::info!(
                            "No metadata table found. Need to create the \
                             schema."
                        );
                        (None, None, true)
                    }
                    _ => return Err(e),
                },
            }
        };

        if need_initialization {
            log::info!("Initializing schema...");
            transaction.execute_batch(Self::SCHEMA_SQL)?;
            transaction.execute(
                "INSERT INTO metadata (key, value) VALUES ('schema_version', \
                 ?1), ('schema_identifier', ?2)",
                params![Self::EXPECTED_VERSION, Self::EXPECTED_IDENTIFIER],
            )?;
            log::info!("Schema initialized successfully.");
        } else if let (Some(v), Some(i)) = (version, identifier) {
            if v == Self::EXPECTED_VERSION && i == Self::EXPECTED_IDENTIFIER {
                log::info!("Database schema is up to date (version {}).", v);
            } else {
                log::error!(
                    "Found existing schema version {} for app {}. Expected \
                     version {} for {}.",
                    v,
                    i,
                    Self::EXPECTED_VERSION,
                    Self::EXPECTED_IDENTIFIER
                );
                return Err(SqliteError::SqliteFailure(
                    rusqlite::ffi::Error::new(1), // 1 is SQLITE_ERROR
                    Some("Schema version mismatch".to_string()),
                ));
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn process_queue_with_result<T>(
        &mut self,
        result_handler: impl FnOnce(&Transaction) -> Result<T, SqliteError>,
    ) -> Result<T, SqliteError> {
        let mut queue = self.operation_queue.lock().unwrap();
        let tx = self.connection.transaction()?;

        while let Some(sql) = queue.pop_front() {
            if sql.trim().to_lowercase().starts_with("select") {
                // For SELECT statements, use query
                tx.query_row(&sql, [], |_| Ok(()))?;
            } else {
                // For other statements (INSERT, UPDATE, DELETE), use execute
                tx.execute(&sql, [])?;
            }
        }
        let result = result_handler(&tx)?;

        tx.commit()?;
        Ok(result)
    }
}
