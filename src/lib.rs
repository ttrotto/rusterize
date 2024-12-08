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
use geo_types::Geometry as GtGeometry;
use py_geo_interface::Geometry;
use numpy::{
    ndarray::{Array2, Array3},
    PyArray3, ToPyArray,
};
use pyo3::{
    prelude::*,
    types::{PyAny, PyString, PyBytes},
};
use pyo3_polars::PyDataFrame;
use polars::prelude::*;
use py_geo_interface::wrappers::f64::AsGeometryVec;
use structs::raster::Raster;

fn process_df(df: &DataFrame) -> () {
    let bad_geom = df.column("geometry").unwrap();
    let good_geom = bad_geom
        .into_iter()
        .map(|polygon| polygon.into())
        .collect();
}

fn rusterize_rust(
    mut df: DataFrame,
    geom: Vec<Geometry>,
    info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    field: String,
    by: String,
) -> Array3<f64> {

}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pydf: PyDataFrame,
    pygeom: PyAny,
    pyinfo: PyAny,
    pypixel_fn: PyString,
    pybackground: PyAny,
    pyfield: Option<PyString>,
    pyby: Option<PyString>,
) -> PyResult<&'py PyArray3<f64>> {
    // extract dataframe
    let mut df = pydf.into()?;

    // extract geometries
    let geom = pygeom.as_geometry_vec()?.0;

    // extract raster information
    let raster_info = Raster::from(&pyinfo);

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
        py.allow_threads(|| rusterize_rust(df, geom, raster_info, pixel_fn, background, field, by));
    let ret = output.to_pyarray(py);
    Ok(ret)
}

fn rusterize(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
