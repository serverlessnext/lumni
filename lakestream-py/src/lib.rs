use std::collections::HashMap;
use std::env;

use lakestream as lakestream_rs;
use lakestream_rs::{
    cli, ListObjectsResult, ObjectStoreHandler, DEFAULT_AWS_REGION,
};
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::{exceptions, ToPyObject};

#[pyclass]
struct Client {
    config: HashMap<String, String>,
}

#[pymethods]
impl Client {
    #[new]
    fn new(region: Option<String>) -> PyResult<Self> {
        let access_key = env::var("AWS_ACCESS_KEY_ID").map_err(|_| {
            PyErr::new::<exceptions::PyValueError, _>(
                "Missing environment variable AWS_ACCESS_KEY_ID",
            )
        })?;
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| {
            PyErr::new::<exceptions::PyValueError, _>(
                "Missing environment variable AWS_SECRET_ACCESS_KEY",
            )
        })?;
        let region = region
            .or_else(|| env::var("AWS_REGION").ok())
            .unwrap_or_else(|| DEFAULT_AWS_REGION.to_string());

        let mut config = HashMap::new();
        config.insert("access_key".to_string(), access_key);
        config.insert("secret_key".to_string(), secret_key);
        config.insert("region".to_string(), region);

        Ok(Client { config })
    }

    fn cli(&self, args: &PyList) -> PyResult<()> {
        let mut args: Vec<String> = args
            .iter()
            .map(|arg| arg.extract::<String>().unwrap())
            .collect();
        args.insert(0, "lakestream".to_string());

        let access_key = env::var("AWS_ACCESS_KEY_ID")
            .expect("Missing environment variable AWS_ACCESS_KEY_ID");
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY")
            .expect("Missing environment variable AWS_SECRET_ACCESS_KEY");
        let mut config = HashMap::new();
        config.insert("access_key".to_string(), access_key);
        config.insert("secret_key".to_string(), secret_key);
        cli::run_cli(args);
        Ok(())
    }

    fn list(
        &self,
        py: Python,
        uri: String,
        max_files: Option<u32>,
    ) -> PyResult<PyObject> {
        // Get the namedtuple function from the collections module
        let collections = py.import("collections")?;
        let namedtuple = collections.getattr("namedtuple")?;

        // Create the FileObject NamedTuple class in Rust
        let file_object_named_tuple =
            namedtuple.call1(("FileObject", ["name", "size", "modified"]))?;

        match ObjectStoreHandler::list_objects(
            uri,
            self.config.clone(),
            max_files,
        ) {
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
        }
    }
}

#[pymodule]
fn lakestream(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Client>()?;
    Ok(())
}
