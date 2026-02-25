mod allocator;
mod geo {
    pub mod edges;
    pub mod parse_geometry;
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
    geo::parse_geometry::ParsedGeometry,
    prelude::*,
    rasterization::{
        pixel_functions::set_pixel_function,
        prepare_dataframe::cast_df,
        rusterize_impl::{Rasterize, RasterizeContext},
    },
};
use geo::raster::{RasterInfo, RawRasterInfo};
use num_traits::Num;
use polars::prelude::DataFrame;
use pyo3::{conversion::FromPyObject, prelude::*, types::PyAny};
use pyo3_polars::PyDataFrame;

macro_rules! dispatch_rusterize {
    (
        $dtype:expr, $encoding:expr, $py:expr, $ctx:expr,
        [ $( ($str_val:pat, $rust_type:ty) ),* ]
    ) => {
        match ($dtype, $encoding) {
            $(
                ($str_val, "xarray" | "numpy") => rusterize_impl::<$rust_type, Dense>($py, $ctx),
                ($str_val, "sparse") => rusterize_impl::<$rust_type, Sparse>($py, $ctx),
            )*
            _ => unimplemented!("Invalid dtype or encoding combination provided."),
        }
    };
}

struct Context<'py> {
    geometry: ParsedGeometry,
    raster_info: RasterInfo,
    pypixel_fn: &'py str,
    pybackground: Option<&'py Bound<'py, PyAny>>,
    df: Option<DataFrame>,
    pyfield: Option<&'py str>,
    pyby: Option<&'py str>,
    pyburn: Option<&'py Bound<'py, PyAny>>,
    opt_flags: OptFlags,
}

fn rusterize_impl<'py, T, R>(py: Python<'py>, ctx: Context<'py>) -> PyResult<PyOut<'py>>
where
    T: Num + Copy + PolarsHandler + Default + PixelOps + for<'a> FromPyObject<'a, 'py>,
    R: Rasterize<T>,
    R::Output: Pythonize,
{
    let background = ctx
        .pybackground
        .and_then(|inner| inner.extract().ok())
        .unwrap_or_default();
    let burn = ctx.pyburn.and_then(|inner| inner.extract().ok()).unwrap_or(T::one());
    let pixel_fn = set_pixel_function(ctx.pypixel_fn);

    // extract column from dataframe (cloning is cheap)
    let casted = cast_df(ctx.df, ctx.pyfield, ctx.pyby, burn, ctx.geometry.len());
    let field = casted.column("field_casted").unwrap().clone();
    let by = casted.column("by_str").ok().and_then(|by| by.str().ok());

    // rasterize
    let rctx = RasterizeContext {
        raster_info: ctx.raster_info,
        geometry: ctx.geometry,
        field,
        pixel_fn,
        background,
        opt_flags: ctx.opt_flags,
    };

    let ret = R::rasterize(rctx, by);
    ret.pythonize(py, ctx.opt_flags)
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
#[pyo3(signature = (geometry, raw_raster_info, pypixel_fn, pydf=None, pyfield=None, pyby=None, pyburn=None, pybackground=None, pytouched=false, pyencoding="xarray", pydtype="float64"))]
#[allow(clippy::too_many_arguments)]
fn rusterize_py<'py>(
    py: Python<'py>,
    geometry: ParsedGeometry,
    raw_raster_info: RawRasterInfo,
    pypixel_fn: &'py str,
    pydf: Option<PyDataFrame>,
    pyfield: Option<&'py str>,
    pyby: Option<&'py str>,
    pyburn: Option<&'py Bound<PyAny>>,
    pybackground: Option<&'py Bound<PyAny>>,
    pytouched: bool,
    pyencoding: &str,
    pydtype: &str,
) -> PyResult<PyOut<'py>> {
    // extract dataframe
    let df: Option<DataFrame> = pydf.map(|inner| inner.into());

    // construct raster info
    let raster_info = RasterInfo::from(raw_raster_info, &geometry);

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

    dispatch_rusterize!(
        pydtype,
        pyencoding,
        py,
        ctx,
        [
            ("uint8", u8),
            ("uint16", u16),
            ("uint32", u32),
            ("uint64", u64),
            ("int8", i8),
            ("int16", i16),
            ("int32", i32),
            ("int64", i64),
            ("float32", f32),
            ("float64", f64)
        ]
    )
}

#[pymodule]
fn rusterize(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
