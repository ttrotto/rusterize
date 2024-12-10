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
use geo_types::Geometry;
use numpy::{
    ndarray::Array3,
    PyArray3, ToPyArray,
};
use polars::prelude::*;
use py_geo_interface::wrappers::f64::AsGeometryVec;
use pyo3::{
    prelude::*,
    types::{PyAny, PyString},
};
use pyo3_polars::PyDataFrame;
use structs::raster::Raster;

fn rusterize_rust(
    mut df: DataFrame,
    geom: Vec<Geometry>,
    ras_info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    field: String,
    by: String,
) -> Array3<f64> {
    // add geometry to dataframe
    let series = Series::new("geometry".into(), geom);
    let df_lazy = df
        .lazy()
        .with_column(series.lit());

    if by.is_empty() {
        // build raster
        let raster = ras_info.build_raster(1);

        // assign values to a single band
        df_lazy.with_columns([
                                 .map(cols([field, "geometry".to_string()]) )
        ])
    } else {
        // build raster
    }
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
