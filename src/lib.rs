mod allocator;
mod geo {
    pub mod edges;
    pub mod from_shapely;
    pub mod raster;
}
mod encoding {
    pub mod arrays;
    mod build_xarray;
    pub mod pyarrays;
    pub mod writers;
}
mod rasterization {
    pub mod burn_geometry;
    pub mod burners;
    pub mod pixel_functions;
    pub mod prepare_dataframe;
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

struct Context<'py> {
    geometry: Vec<Geometry>,
    raster_info: RasterInfo,
    pypixel_fn: &'py str,
    pybackground: Option<&'py Bound<'py, PyAny>>,
    df: Option<DataFrame>,
    pyfield: Option<&'py str>,
    pyby: Option<&'py str>,
    pyburn: Option<&'py Bound<'py, PyAny>>,
    opt_flags: OptFlags,
}

#[allow(clippy::too_many_arguments)]
fn execute_rusterize<'py, T, R>(py: Python<'py>, ctx: Context<'py>) -> PyResult<PyOut<'py>>
where
    T: Num + Copy + PixelOps + PolarsHandler + FromPyObject<'py> + Element + Default + 'static,
    R: Rasterize<T>,
    R::Output: Pythonize,
{
    let background = ctx
        .pybackground
        .and_then(|inner| inner.extract().ok())
        .unwrap_or_default();
    let burn = ctx.pyburn.and_then(|inner| inner.extract().ok()).unwrap_or(T::one());

    let pixel_fn = set_pixel_function(ctx.pypixel_fn);

    // rusterize
    let ret = rusterize_impl::<T, R>(
        ctx.geometry,
        ctx.raster_info,
        pixel_fn,
        background,
        ctx.df,
        ctx.pyfield,
        ctx.pyby,
        burn,
        ctx.opt_flags,
    );
    ret.pythonize(py, ctx.opt_flags)
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
#[pyo3(signature = (pygeometry, pyinfo, pypixel_fn, pydf=None, pyfield=None, pyby=None, pyburn=None, pybackground=None, pytouched=false, pyencoding="xarray", pydtype="float64"))]
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
    pytouched: bool,
    pyencoding: &str,
    pydtype: &str,
) -> PyResult<PyOut<'py>> {
    // extract dataframe
    let df: Option<DataFrame> = pydf.map(|inner| inner.into());

    // parse geometries
    let geometry = from_shapely(py, pygeometry)?;

    // extract raster information
    let raster_info = RasterInfo::extract_bound(pyinfo)?;

    // optional runtime flags
    let opt_flags = OptFlags::new(pytouched, pyencoding, pypixel_fn);

    let ctx = Context {
        geometry,
        raster_info,
        pypixel_fn,
        pybackground,
        df,
        pyfield,
        pyby,
        pyburn,
        opt_flags,
    };

    match (pydtype, pyencoding) {
        ("uint8", "xarray" | "numpy") => execute_rusterize::<u8, Dense>(py, ctx),
        ("uint8", "sparse") => execute_rusterize::<u8, Sparse>(py, ctx),

        ("uint16", "xarray" | "numpy") => execute_rusterize::<u16, Dense>(py, ctx),
        ("uint16", "sparse") => execute_rusterize::<u16, Sparse>(py, ctx),

        ("uint32", "xarray" | "numpy") => execute_rusterize::<u32, Dense>(py, ctx),
        ("uint32", "sparse") => execute_rusterize::<u32, Sparse>(py, ctx),

        ("uint64", "xarray" | "numpy") => execute_rusterize::<u64, Dense>(py, ctx),
        ("uint64", "sparse") => execute_rusterize::<u64, Sparse>(py, ctx),

        ("int8", "xarray" | "numpy") => execute_rusterize::<i8, Dense>(py, ctx),
        ("int8", "sparse") => execute_rusterize::<i8, Sparse>(py, ctx),

        ("int16", "xarray" | "numpy") => execute_rusterize::<i16, Dense>(py, ctx),
        ("int16", "sparse") => execute_rusterize::<i16, Sparse>(py, ctx),

        ("int32", "xarray" | "numpy") => execute_rusterize::<i32, Dense>(py, ctx),
        ("int32", "sparse") => execute_rusterize::<i32, Sparse>(py, ctx),

        ("int64", "xarray" | "numpy") => execute_rusterize::<i64, Dense>(py, ctx),
        ("int64", "sparse") => execute_rusterize::<i64, Sparse>(py, ctx),

        ("float32", "xarray" | "numpy") => execute_rusterize::<f32, Dense>(py, ctx),
        ("float32", "sparse") => execute_rusterize::<f32, Sparse>(py, ctx),

        ("float64", "xarray" | "numpy") => execute_rusterize::<f64, Dense>(py, ctx),
        ("float64", "sparse") => execute_rusterize::<f64, Sparse>(py, ctx),

        _ => unimplemented!(
            "`dtype` must be one of uint8, uint16, uint32, uint64, int8, int16, int32, int64, float32, float64; \
             and `encoding` must be either 'xarray', 'numpy', or 'sparse'"
        ),
    }
}

#[pymodule]
fn rusterize(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
