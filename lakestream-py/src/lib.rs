use std::env;

// start with :: to ensure local crate is used
use ::lakestream::{
    ListObjectsResult, ObjectStoreHandler, FileObjectFilter, Config, AWS_DEFAULT_REGION,
};
use ::lakestream_cli::run_cli;

use tokio::runtime::Runtime;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyDict,PyAny};
use pyo3::{exceptions, ToPyObject};


#[pyclass]
struct _Client {
    config: Config,
}

#[pymethods]
impl _Client {
    #[new]
    fn new(region: Option<String>) -> PyResult<Self> {
        let region = region
            .or_else(|| env::var("AWS_REGION").ok())
            .unwrap_or_else(|| AWS_DEFAULT_REGION.to_string());
        let config = Config::with_setting("region".to_string(), region);
        Ok(_Client { config })
    }

    fn cli(&self, args: &PyList) -> PyResult<()> {
        let mut args: Vec<String> = args
            .iter()
            .map(|arg| arg.extract::<String>().unwrap())
            .collect();
        args.insert(0, "lakestream".to_string());
        run_cli(args);
        Ok(())
    }

    fn list(
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
        let result = rt.block_on( handler.list_objects(
            uri,
            self.config.clone(),
            recursive.unwrap_or(false),
            max_files,
            &filter,
            None,
        ));

        match result {
            Ok(None) => Ok(PyList::empty(py).to_object(py)),
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
                ListObjectsResult::Buckets(buckets) => {
                    let py_buckets = buckets
                        .into_iter()
                        .map(|bucket| bucket.name().to_owned())
                        .collect::<Vec<_>>();
                    Ok(PyList::new(py, &py_buckets).to_object(py))
                }
            },
            Err(err) => Err(PyErr::new::<exceptions::PyValueError, _>(format!(
                "Error listing objects: {}",
                err
            ))),
        }
    }
}

#[pymodule]
fn lakestream(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<_Client>()?;
    Ok(())
}


fn create_filter(py: Python, filter_dict: Option<&PyDict>) -> PyResult<Option<FileObjectFilter>> {
    // Create the filter from the dictionary
    let filter = filter_dict.as_ref().map(|filter_dict| {
        let filter_name = extract_first_value(py, filter_dict.get_item("name"));
        let filter_size = extract_first_value(py, filter_dict.get_item("size"));
        let filter_mtime = extract_first_value(py, filter_dict.get_item("mtime"));

        FileObjectFilter::new(
            filter_name.as_deref(),
            filter_size.as_deref(),
            filter_mtime.as_deref(),
        )
    });

    let filter = match filter {
        Some(Ok(filter)) => Some(filter),
        Some(Err(err)) => {
            return Err(PyErr::new::<
                exceptions::PyValueError,
                _,
            >(format!("Error creating filter: {}", err)))
        }
        None => None,
    };
    PyResult::Ok(filter)
}

fn extract_first_value(_py: Python, value: Option<&PyAny>) -> Option<String> {
    if let Some(value) = value {
        if let Ok(s) = value.extract::<String>() {
            Some(s)
        } else if let Ok(list) = value.extract::<Vec<String>>() {
            list.first().cloned()
        } else {
            None
        }
    } else {
        None
    }
}

