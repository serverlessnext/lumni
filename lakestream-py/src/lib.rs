
mod client;
mod utils;

// start with :: to ensure local crate is used

use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(lakestream, LakestreamError, PyException, "An error occurred in the Lakestream library.");



#[pymodule]
fn lakestream(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<client::_Client>()?;
    Ok(())
}


