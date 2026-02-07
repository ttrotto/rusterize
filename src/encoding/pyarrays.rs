/* Python conversion traits and wrappers */

use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;
use std::sync::Arc;

use crate::prelude::OptFlags;

#[derive(IntoPyObject)]
pub enum PyOut<'py> {
    Dense(Bound<'py, PyAny>),
    Sparse(PySparseArray),
}

pub trait Pythonize {
    // convert rusterization output into python object
    fn pythonize(self, py: Python, opt_flags: OptFlags) -> PyResult<PyOut>;
}

pub trait PySparseArrayTraits: Send + Sync {
    fn size_str(&self) -> String;
    fn shape(&self) -> (&usize, &usize);
    fn resolution(&self) -> (&f64, &f64);
    fn extent(&self) -> (&f64, &f64, &f64, &f64);
    fn epsg(&self) -> &Option<u16>;
    fn to_xarray<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>>;
    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>>;
    fn to_frame(&self) -> PyDataFrame;
}

#[pyclass(name = "SparseArray")]
pub struct PySparseArray(pub Arc<dyn PySparseArrayTraits>);

#[pymethods]
impl PySparseArray {
    fn __repr__(&self) -> String {
        let epsg = if let Some(epsg) = self.0.epsg() {
            epsg.to_string()
        } else {
            String::from("None")
        };

        format!(
            "SparseArray:\n- Shape: {:?}\n- Extent: {:?}\n- Resolution: {:?}\n- EPSG: {}\n- Estimated size: {}",
            self.0.shape(),
            self.0.extent(),
            self.0.resolution(),
            epsg,
            self.0.size_str()
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
