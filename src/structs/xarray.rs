/*
Simple output structure to build a xarray in Python from a dictionary.
Python will not build a xarray internally because it copies data from numpy.
 */

use dict_derive::IntoPyObject;
use numpy::{PyArray1, PyArray3};
use pyo3::{prelude::*, types::PyList};

#[derive(IntoPyObject)]
pub struct Coordinates<'py> {
    pub x: Bound<'py, PyArray1<f64>>,
    pub y: Bound<'py, PyArray1<f64>>,
    pub bands: Bound<'py, PyList>,
}

impl<'py> Coordinates<'py> {
    fn new(x: Bound<'py, PyArray1<f64>>, y: Bound<'py, PyArray1<f64>>, bands: Bound<'py, PyList>) -> Self {
        Self {x, y, bands}
    }
}

#[derive(IntoPyObject)]
pub struct Xarray<'py> {
    pub data: Bound<'py, PyArray3<f64>>,
    pub dims: Bound<'py, PyList>,
    pub coords: Coordinates<'py>,
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
