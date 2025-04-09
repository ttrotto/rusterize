/*
Build dictionary for xarray construction.
In some cases Python will build a xarray without copying the Rust array.
 */

use numpy::{PyArray1, PyArray3};
use pyo3::{
    prelude::*,
    types::{PyDict, PyList},
};

pub fn build_xarray<'py>(
    py: Python<'py>,
    data: Bound<'py, PyArray3<f64>>,
    dims: Bound<'py, PyList>,
    x: Bound<'py, PyArray1<f64>>,
    y: Bound<'py, PyArray1<f64>>,
    bands: Bound<'py, PyList>,
) -> PyResult<Bound<'py, PyDict>> {
    // dimensions
    let dim_x = PyDict::new(py);
    dim_x.set_item("dims", "x")?;
    dim_x.set_item("data", x)?;

    let dim_y = PyDict::new(py);
    dim_y.set_item("dims", "y")?;
    dim_y.set_item("data", y)?;

    let dim_bands = PyDict::new(py);
    dim_bands.set_item("dims", "bands")?;
    dim_bands.set_item("data", bands)?;

    // coordinates
    let coords = PyDict::new(py);
    coords.set_item("x", dim_x)?;
    coords.set_item("y", dim_y)?;
    coords.set_item("bands", dim_bands)?;

    // xarray
    let xarray = PyDict::new(py);
    xarray.set_item("data", data)?;
    xarray.set_item("dims", dims)?;
    xarray.set_item("coords", coords)?;
    Ok(xarray)
}
