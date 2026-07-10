use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;
use std::sync::Arc;

use super::xarray::build_xarray;
use crate::prelude::OptionalFlags;
use num_traits::Num;
use numpy::{Element, IntoPyArray};
use rusterize::prelude::*;

#[derive(IntoPyObject)]
pub enum PyOutput<'py> {
    Dense(Bound<'py, PyAny>),
    Sparse(PySparseArray),
}

/// Convert a [`rusterize::DenseArray`] or a [`rusterize::SparseArray`] into python object
pub trait Pythonize {
    fn pythonize(self, py: Python, opt_flags: OptionalFlags) -> PyResult<PyOutput>;
}

impl<N> Pythonize for DenseArray<N>
where
    N: Num + Element,
{
    fn pythonize(self, py: Python, opt_flags: OptionalFlags) -> PyResult<PyOutput> {
        let (array, band_names, raster_info) = self.into_parts();
        let data = array.into_pyarray(py);

        if opt_flags.xarray {
            let xarray = build_xarray(py, &raster_info, data, &band_names)?;
            Ok(PyOutput::Dense(xarray))
        } else {
            Ok(PyOutput::Dense(data.into_any()))
        }
    }
}

impl<N> Pythonize for SparseArray<N>
where
    N: RasterDtype + Element + 'static,
{
    fn pythonize(self, _py: Python, _opt_flags: OptionalFlags) -> PyResult<PyOutput> {
        Ok(PyOutput::Sparse(PySparseArray(Arc::new(self))))
    }
}

/// Trait to convert a [`rusterize::SparseArray`] into a python object that mask the output data type.
pub trait PySparseArrayTraits: Send + Sync {
    fn shape(&self) -> (usize, usize, usize);
    fn extent(&self) -> (f64, f64, f64, f64);
    fn resolution(&self) -> (f64, f64);
    fn epsg(&self) -> Option<u16>;
    fn size_hint(&self) -> String;
    fn to_xarray<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>>;
    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>>;
    fn to_frame(&self) -> PyDataFrame;
}

impl<T> PySparseArrayTraits for SparseArray<T>
where
    T: RasterDtype + Element,
{
    fn shape(&self) -> (usize, usize, usize) {
        self.shape()
    }
    fn extent(&self) -> (f64, f64, f64, f64) {
        self.extent()
    }
    fn resolution(&self) -> (f64, f64) {
        self.resolution()
    }
    fn epsg(&self) -> Option<u16> {
        self.epsg()
    }

    /// Estimated size of the materialized [`rusterize::DenseArray`]
    fn size_hint(&self) -> String {
        let (nbands, nrows, ncols) = self.shape();
        let bytes = std::mem::size_of::<T>() * nbands * nrows * ncols;
        if bytes < 1000 {
            format!("{} bytes", bytes)
        } else if bytes < 1_000_000 {
            format!("{:.2} KB", bytes as f32 / 1000.0)
        } else if bytes < 1_000_000_000 {
            format!("{:.2} MB", bytes as f32 / 1_000_000.0)
        } else {
            format!("{:.2} GB", bytes as f32 / 1_000_000_000.0)
        }
    }

    fn to_xarray<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let raster = self.build_array();
        let data = raster.into_pyarray(py);
        build_xarray(py, self.raster_info(), data, self.band_names())
    }

    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let raster = self.build_array();
        Ok(raster.into_pyarray(py).into_any())
    }

    fn to_frame(&self) -> PyDataFrame {
        PyDataFrame(self.to_frame())
    }
}

#[pyclass(name = "SparseArray", frozen)]
pub struct PySparseArray(pub Arc<dyn PySparseArrayTraits>);

#[pymethods]
impl PySparseArray {
    fn __repr__(&self) -> String {
        let epsg = match self.0.epsg() {
            Some(e) => e.to_string(),
            None => String::from("None"),
        };

        format!(
            "SparseArray:\n- Shape: {:?}\n- Extent: {:?}\n- Resolution: {:?}\n- EPSG: {}\n- Estimated size: {}",
            self.0.shape(),
            self.0.extent(),
            self.0.resolution(),
            epsg,
            self.0.size_hint()
        )
    }

    fn to_xarray<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.0.to_xarray(py)
    }

    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.0.to_numpy(py)
    }

    fn to_frame(&self) -> PyDataFrame {
        self.0.to_frame()
    }
}
