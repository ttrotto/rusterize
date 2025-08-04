mod allocator;
mod structs {
    pub mod edge;
    pub mod raster;
}
mod geom {
    pub mod from_shapely;
    pub mod validate;
}
mod edge_collection;
mod pixel_functions;
mod prelude;
mod rasterize_geometry;
mod rusterize_impl;
mod to_xarray;

use crate::{
    geom::from_shapely::from_shapely, pixel_functions::set_pixel_function, prelude::*,
    rusterize_impl::rusterize_impl,
};
use geo_types::Geometry;
use num_traits::{Num, NumCast};
use numpy::Element;
use polars::prelude::DataFrame;
use pyo3::{
    prelude::*,
    types::{PyAny, PyDict},
};
use pyo3_polars::PyDataFrame;
use structs::raster::RasterInfo;
use to_xarray::build_xarray;

#[allow(clippy::too_many_arguments)]
fn execute_rusterize<'py, T>(
    py: Python<'py>,
    geometry: Vec<Geometry>,
    mut raster_info: RasterInfo,
    pypixel_fn: &str,
    pybackground: Option<&Bound<'py, PyAny>>,
    df: Option<DataFrame>,
    pyfield: Option<&str>,
    pyby: Option<&str>,
    pyburn: Option<f64>,
) -> PyResult<Bound<'py, PyDict>>
where
    T: Num + NumCast + Copy + PixelOps + PolarsHandler + FromPyObject<'py> + Element + Default,
{
    let background = pybackground
        .and_then(|inner| inner.extract::<T>().ok())
        .unwrap_or_default();
    let burn = pyburn.and_then(|v| T::from(v)).unwrap_or(T::one());
    let pixel_fn = set_pixel_function::<T>(pypixel_fn);

    // rusterize
    let (ret, band_names) = rusterize_impl::<T>(
        geometry,
        &mut raster_info,
        pixel_fn,
        background,
        df,
        pyfield,
        pyby,
        burn,
    );

    // build xarray dictionary
    let xarray = build_xarray(py, raster_info, ret, band_names)?;
    Ok(xarray)
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
#[pyo3(signature = (pygeometry, pyinfo, pypixel_fn, pydf=None, pyfield=None, pyby=None, pyburn=None, pybackground=None, pydtype="float64"))]
#[allow(clippy::too_many_arguments)]
fn rusterize_py<'py>(
    py: Python<'py>,
    pygeometry: &Bound<'py, PyAny>,
    pyinfo: &Bound<'py, PyAny>,
    pypixel_fn: &str,
    pydf: Option<PyDataFrame>,
    pyfield: Option<&str>,
    pyby: Option<&str>,
    pyburn: Option<f64>,
    pybackground: Option<&Bound<'py, PyAny>>,
    pydtype: &str,
) -> PyResult<Bound<'py, PyDict>> {
    // extract dataframe
    let df: Option<DataFrame> = pydf.map(|inner| inner.into());

    // parse geometries
    let geometry = from_shapely(py, pygeometry)?;

    // extract raster information
    let raster_info = RasterInfo::from(pyinfo);

    // branch
    match pydtype {
        "uint8" => execute_rusterize::<u8>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "uint16" => execute_rusterize::<u16>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "uint32" => execute_rusterize::<u32>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "uint64" => execute_rusterize::<u64>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "int8" => execute_rusterize::<i8>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "int16" => execute_rusterize::<i16>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "int32" => execute_rusterize::<i32>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "int64" => execute_rusterize::<i64>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "float32" => execute_rusterize::<f32>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        "float64" => execute_rusterize::<f64>(
            py,
            geometry,
            raster_info,
            pypixel_fn,
            pybackground,
            df,
            pyfield,
            pyby,
            pyburn,
        ),
        _ => unimplemented!(
            "`dtype` must be a one of uint8, uint16, uint32, uint64, int8, int16, int32, int64, float32, float64"
        ),
    }
}

#[pymodule]
fn rusterize(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
