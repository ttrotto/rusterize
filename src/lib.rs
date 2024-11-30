#![feature(extract_if)]
extern crate blas_src;

mod structs {
    pub mod edge;
    pub mod raster;
}
mod edgelist;
mod pixel_functions;
mod rasterize_polygon;

use std::ops::Deref;
use geo::Geometry;
use pyo3::{
    prelude::*,
    types::{PyList, PyString, PyFloat, PyAny}
};

// #[pyfunction]
// #[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    polygons_py: PyList,
    field_vals_py: PyList,
    by: PyString,
    res: PyAny,
    pixel_fn: &PyString,
    background: &PyFloat,
) -> PyResult<&'py PyAny> {
    // convert python shapes list into vector of geometries
    let shape_count = polygons_py.len();
    let mut shapes: Vec<Geometry<f64>> = Vec::with_capacity(shape_count);
    for i in 0..shape_count {
        let polygon_vals = polygons_py[i];
    }
}