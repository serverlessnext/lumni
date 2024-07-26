
mod client;
mod utils;

// start with :: to ensure local crate is used

use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(lumni, InternalError, PyException, "An error occurred in the lumni library.");



#[pymodule]
fn lumni(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<client::_Client>()?;
    Ok(())
}


