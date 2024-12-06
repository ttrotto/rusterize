#![feature(extract_if)]
extern crate blas_src;

mod structs {
    pub mod edge;
    pub mod raster;
}
mod edgelist;
mod pixel_functions;
mod rasterize_polygon;

use crate::pixel_functions::{set_pixel_function, PixelFn};
use geo::Geometry;
use numpy::{
    ndarray::{Array2, Array3},
    PyArray3, ToPyArray,
};
use polars::prelude::*;
use pyo3::{
    prelude::*,
    types::{PyAny, PyString},
};
use pyo3_polars::PyDataFrame;
use structs::raster::Raster;

fn rusterize_rust(
    df: DataFrame,
    info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    field: String,
    by: String,
) -> Array3<f64> {
    // extract geometries
    let fgeom = df
        .column("geometry").unwrap()
        .

    if by.is_empty() {
        // no group by

    } else {
        // groyp by
    }
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pydf: PyDataFrame,
    pyinfo: PyAny,
    pypixel_fn: PyString,
    pybackground: PyAny,
    pyfield: Option<PyString>,
    pyby: Option<PyString>,
) -> PyResult<&'py PyArray3<f64>> {
    // extract polars dataframe
    let mut df = pydf.into();

    // extract raster information
    let (xmin, ymin, xmax, ymax, xres, yres): (f64, f64, f64, f64, f64, f64) = (
        pyinfo.getattr("xmin")?.extract()?,
        pyinfo.getattr("ymin")?.extract()?,
        pyinfo.getattr("xmax")?.extract()?,
        pyinfo.getattr("ymax")?.extract()?,
        pyinfo.getattr("xres")?.extract()?,
        pyinfo.getattr("yres")?.extract()?,
    );
    let raster_info = Raster::new(xmin, xmax, ymin, ymax, xres, yres);

    // extract function arguments
    let fun = pypixel_fn.to_str()?;
    let pixel_fn = set_pixel_function(fun)?;
    let background = pybackground.extract::<f64>()?;
    let field = match pyfield {
        Some(inner) => inner.to_string(),
        None => String::new(),
    };
    let by = match pyby {
        Some(inner) => inner.to_string(),
        None => String::new(),
    };

    // rusterize
    let output =
        py.allow_threads(|| rusterize_rust(df, raster_info, pixel_fn, background, field, by));
    let ret = output.to_pyarray(py);
    Ok(ret)
}

fn rusterize(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
