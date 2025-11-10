mod allocator;
mod geo {
    pub mod edge;
    pub mod edge_collection;
    pub mod from_shapely;
    pub mod raster;
    pub mod validate;
}
mod encoding {
    pub mod arrays;
    mod build_xarray;
    pub mod pyarrays;
    pub mod writers;
}
mod rasterization {
    pub mod pixel_functions;
    pub mod prepare_dataframe;
    pub mod rasterize_geometry;
    pub mod rusterize_impl;
}
mod prelude;

use crate::{
    encoding::pyarrays::{PyOut, Pythonize},
    geo::from_shapely::from_shapely,
    prelude::*,
    rasterization::{
        pixel_functions::set_pixel_function,
        rusterize_impl::{Rasterize, rusterize_impl},
    },
};
use geo::raster::RasterInfo;
use geo_types::Geometry;
use num_traits::Num;
use numpy::Element;
use polars::prelude::DataFrame;
use pyo3::{prelude::*, types::PyAny};
use pyo3_polars::PyDataFrame;

struct Metadata<'py> {
    geometry: Vec<Geometry>,
    raster_info: RasterInfo,
    pypixel_fn: &'py str,
    pybackground: Option<&'py Bound<'py, PyAny>>,
    df: Option<DataFrame>,
    pyfield: Option<&'py str>,
    pyby: Option<&'py str>,
    pyburn: Option<&'py Bound<'py, PyAny>>,
}

#[allow(clippy::too_many_arguments)]
fn execute_rusterize<'py, T, R>(py: Python<'py>, meta: Metadata<'py>) -> PyResult<PyOut<'py>>
where
    T: Num + Copy + PixelOps + PolarsHandler + FromPyObject<'py> + Element + Default + 'static,
    R: Rasterize<T>,
    R::Output: Pythonize,
{
    let background = meta
        .pybackground
        .and_then(|inner| inner.extract().ok())
        .unwrap_or_default();
    let burn = meta
        .pyburn
        .and_then(|inner| inner.extract().ok())
        .unwrap_or(T::one());
    let pixel_fn = set_pixel_function(meta.pypixel_fn);

    // rusterize
    let array = rusterize_impl::<T, R>(
        meta.geometry,
        meta.raster_info,
        pixel_fn,
        background,
        meta.df,
        meta.pyfield,
        meta.pyby,
        burn,
    );
    array.pythonize(py)
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
#[pyo3(signature = (pygeometry, pyinfo, pypixel_fn, pydf=None, pyfield=None, pyby=None, pyburn=None, pybackground=None, pyencoding="dense", pydtype="float64"))]
#[allow(clippy::too_many_arguments)]
fn rusterize_py<'py>(
    py: Python<'py>,
    pygeometry: &Bound<'py, PyAny>,
    pyinfo: &Bound<'py, PyAny>,
    pypixel_fn: &'py str,
    pydf: Option<PyDataFrame>,
    pyfield: Option<&'py str>,
    pyby: Option<&'py str>,
    pyburn: Option<&'py Bound<'py, PyAny>>,
    pybackground: Option<&'py Bound<'py, PyAny>>,
    pyencoding: &str,
    pydtype: &str,
) -> PyResult<PyOut<'py>> {
    // extract dataframe
    let df: Option<DataFrame> = pydf.map(|inner| inner.into());

    // parse geometries
    let geometry = from_shapely(py, pygeometry)?;

    // extract raster information
    let raster_info = RasterInfo::from(pyinfo);

    let meta = Metadata {
        geometry,
        raster_info,
        pypixel_fn,
        pybackground,
        df,
        pyfield,
        pyby,
        pyburn,
    };

    match (pydtype, pyencoding) {
        ("uint8", "dense") => execute_rusterize::<u8, Dense>(py, meta),
        ("uint8", "sparse") => execute_rusterize::<u8, Sparse>(py, meta),

        ("uint16", "dense") => execute_rusterize::<u16, Dense>(py, meta),
        ("uint16", "sparse") => execute_rusterize::<u16, Sparse>(py, meta),

        ("uint32", "dense") => execute_rusterize::<u32, Dense>(py, meta),
        ("uint32", "sparse") => execute_rusterize::<u32, Sparse>(py, meta),

        ("uint64", "dense") => execute_rusterize::<u64, Dense>(py, meta),
        ("uint64", "sparse") => execute_rusterize::<u64, Sparse>(py, meta),

        ("int8", "dense") => execute_rusterize::<i8, Dense>(py, meta),
        ("int8", "sparse") => execute_rusterize::<i8, Sparse>(py, meta),

        ("int16", "dense") => execute_rusterize::<i16, Dense>(py, meta),
        ("int16", "sparse") => execute_rusterize::<i16, Sparse>(py, meta),

        ("int32", "dense") => execute_rusterize::<i32, Dense>(py, meta),
        ("int32", "sparse") => execute_rusterize::<i32, Sparse>(py, meta),

        ("int64", "dense") => execute_rusterize::<i64, Dense>(py, meta),
        ("int64", "sparse") => execute_rusterize::<i64, Sparse>(py, meta),

        ("float32", "dense") => execute_rusterize::<f32, Dense>(py, meta),
        ("float32", "sparse") => execute_rusterize::<f32, Sparse>(py, meta),

        ("float64", "dense") => execute_rusterize::<f64, Dense>(py, meta),
        ("float64", "sparse") => execute_rusterize::<f64, Sparse>(py, meta),

        _ => unimplemented!(
            "`dtype` must be one of uint8, uint16, uint32, uint64, int8, int16, int32, int64, float32, float64; \
             and `encoding` must be either 'dense' or 'sparse'"
        ),
    }
}

#[pymodule]
fn rusterize(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
