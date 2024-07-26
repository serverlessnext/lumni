
use std::collections::HashMap;
use std::env;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyDict, PyBytes};

use crate::InternalError;
use crate::utils::create_filter;
use tokio::runtime::Runtime;

// start with :: to ensure local crate is used
use ::lumni::{ListObjectsResult, ObjectStoreHandler, EnvironmentConfig, AWS_DEFAULT_REGION};
use ::lumni_cli::run_cli;

#[pyclass]
pub struct _Client {
    config: EnvironmentConfig,
}

#[pymethods]
impl _Client {
    #[new]
    fn new(region: Option<String>) -> PyResult<Self> {
        let region = region
            .or_else(|| env::var("AWS_REGION").ok())
            .unwrap_or_else(|| AWS_DEFAULT_REGION.to_string());

        let mut config = HashMap::new();
        config.insert("region".to_string(), region);
        let config = EnvironmentConfig {
            settings: config,
        };
        Ok(_Client { config })
    }

    fn cli(&self, args: &PyList) -> PyResult<()> {
        let mut args: Vec<String> = args
            .iter()
            .map(|arg| arg.extract::<String>().unwrap())
            .collect();
        args.insert(0, "lumni".to_string());
        run_cli(args);
        Ok(())
    }

    fn list_objects(
        &self,
        py: Python,
        uri: String,
        recursive: Option<bool>,
        max_files: Option<u32>,
        filter_dict: Option<&PyDict>,
    ) -> PyResult<PyObject> {
        // Get the namedtuple function from the collections module
        let collections = py.import("collections")?;
        let namedtuple = collections.getattr("namedtuple")?;

        // Create the FileObject NamedTuple class in Rust
        let file_object_named_tuple =
        namedtuple.call1(("FileObject", ["name", "size", "modified"]))?;

        // Create the filter from the dictionary
        let filter = create_filter(py, filter_dict)?;

        // Create a new Tokio runtime
        let rt = Runtime::new().unwrap();

        // Call the async function and block on it to get the result
        let handler = ObjectStoreHandler::new(None);
        let result = rt.block_on(handler.list_objects(
            &uri,
            &self.config,
            recursive.unwrap_or(false),
            max_files,
            &filter,
            None,
        ));

        match result {
            Ok(Some(list_objects_result)) => match list_objects_result {
                ListObjectsResult::FileObjects(file_objects) => {
                    let py_file_objects = file_objects
                        .into_iter()
                        .map(|fo| {
                            // Create instances of the FileObject NamedTuple
                            file_object_named_tuple.call1((
                                fo.name(),
                                fo.size(),
                                fo.modified().unwrap_or_default(),
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()?; // Collect the PyResult values into a single Result
                    Ok(PyList::new(py, &py_file_objects).to_object(py))
                }
                _ => {
                    let lumni_error = InternalError::new_err(format!("Error listing objects: {}", "Unknown error"));
                    Err(lumni_error)
                },
            },
            Ok(None) => Ok(PyList::empty(py).to_object(py)),
            Err(err) => {
                let lumni_error = InternalError::new_err(format!("Error listing objects: {}", err));
                Err(lumni_error)
            },
        }
    }

    fn list_buckets(
        &self,
        py: Python,
        uri: String,
    ) -> PyResult<PyObject> {
        // Create a new Tokio runtime
        let rt = Runtime::new().unwrap();

        // Call the async function and block on it to get the result
        let handler = ObjectStoreHandler::new(None);
        // let uri = format!("{}://", self.config.scheme());
        let result = rt.block_on(handler.list_buckets(
            &uri,
            &self.config,
            None,
        ));

        match result {
            Ok(Some(list_objects_result)) => match list_objects_result {
                ListObjectsResult::Buckets(buckets) => {
                    let py_buckets = buckets
                        .into_iter()
                        .map(|bucket| bucket.name().to_owned())
                        .collect::<Vec<_>>();
                    Ok(PyList::new(py, &py_buckets).to_object(py))
                }
                _ => {
                    let lumni_error = InternalError::new_err(format!("Error listing buckets"));
                    Err(lumni_error)
                },
            },
            Ok(None) => Ok(PyList::empty(py).to_object(py)),
            Err(err) => {
                let lumni_error = InternalError::new_err(format!("Error listing buckets: {}", err));
                Err(lumni_error)
            },
        }
    }

    fn get_object(&self, py: Python, uri: String) -> PyResult<PyObject> {
        // Create a new Tokio runtime
        let rt = Runtime::new().unwrap();

        // Call the async function and block on it to get the result
        let handler = ObjectStoreHandler::new(None);
        let result = rt.block_on(handler.get_object(&uri, &self.config, None));

        match result {
            Ok(Some(data)) => Ok(PyBytes::new(py, &data).to_object(py)),
            Ok(None) => Err(InternalError::new_err("No data received")),
            Err(err) => {
                let lumni_error = InternalError::new_err(format!("Error getting object: {}", err));
                Err(lumni_error)
            },
        }
    }
}
