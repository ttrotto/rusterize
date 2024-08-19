/*
Check support for PyObjects.
 */

use pyo3::{prelude::*, exceptions::PyTypeError};
use pyo3::types::{PyAny, PyList, PyString};

pub fn is_ready(py_obj: &PyAny,
                field: &PyAny) -> PyResult<bool> {
    // check that input geometry is a list
    if !py_obj.is_instance_of::<PyList>() {
        return Err(PyTypeError::new_err("Only geometry list objects are supported."))
    }

    // check that input field is non-empty
    if !field.is_instance_of::<PyString>() {
        return Err(PyTypeError::new_err("Field must be a string."))
    } else {
        if field.extract::<String>()?.is_empty() {
            return Err(PyTypeError::new_err("Field to rasterize is empty."))
        }
    }

    Ok(true)
}

