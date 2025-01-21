/*
Simple output structure to build a xarray in Python from a dictionary.
Python will not build a xarray internally because it copies data from numpy.
 */

use dict_derive::IntoPyObject;
use numpy::{PyArray1, PyArray3};
use pyo3::{prelude::*, types::PyList};

#[derive(IntoPyObject)]
struct Dims<'py> {
    dims: &'py str,
    data: Bound<'py, PyAny>,
}

impl<'py> Dims<'py> {
    fn new(dims: &'py str, data: Bound<'py, PyAny>) -> Self {
        Self { dims, data }
    }
}

#[derive(IntoPyObject)]
struct Coordinates<'py> {
    x: Dims<'py>,
    y: Dims<'py>,
    bands: Dims<'py>,
}

impl<'py> Coordinates<'py> {
    fn new(
        x: Bound<'py, PyArray1<f64>>,
        y: Bound<'py, PyArray1<f64>>,
        bands: Bound<'py, PyList>,
    ) -> Self {
        let x = Dims::new("x", x.into_any());
        let y = Dims::new("y", y.into_any());
        let bands = Dims::new("bands", bands.into_any());
        Self { x, y, bands }
    }
}

#[derive(IntoPyObject)]
pub struct Xarray<'py> {
    data: Bound<'py, PyArray3<f64>>,
    dims: Bound<'py, PyList>,
    coords: Coordinates<'py>,
}

impl<'py> Xarray<'py> {
    pub fn build_xarray(
        data: Bound<'py, PyArray3<f64>>,
        dims: Bound<'py, PyList>,
        x: Bound<'py, PyArray1<f64>>,
        y: Bound<'py, PyArray1<f64>>,
        bands: Bound<'py, PyList>,
    ) -> Self {
        let coords = Coordinates::new(x, y, bands);
        Self { data, dims, coords }
    }
}
