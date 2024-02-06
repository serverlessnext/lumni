
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyAny};
use pyo3::exceptions;
use ::xlatti::FileObjectFilter;


pub fn create_filter(py: Python, filter_dict: Option<&PyDict>) -> PyResult<Option<FileObjectFilter>> {
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

