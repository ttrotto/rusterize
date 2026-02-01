/*
Build xarray object from a dictionary.

The xarray module is passed as a function argument to avoid importing
it twice for DenseSparse and SparseArray

*/

use crate::geo::raster::RasterInfo;
use num_traits::Num;
use numpy::{Element, PyArray3};
use pyo3::{
    prelude::*,
    types::{PyDict, PyList},
};

pub fn build_xarray<'py, T>(
    py: Python<'py>,
    xarray_module: Bound<'py, PyModule>,
    raster_info: RasterInfo,
    data: Bound<'py, PyArray3<T>>,
    band_names: Vec<String>,
) -> PyResult<Bound<'py, PyAny>>
where
    T: Num + Element,
{
    let (y, x) = raster_info.make_coordinates(py);
    let bands = PyList::new(py, band_names)?;
    let dims = PyList::new(py, vec!["bands", "y", "x"])?;

    // dimensions
    let dim_x = PyDict::new(py);
    dim_x.set_item("dims", "x")?;
    dim_x.set_item("data", x)?;

    let dim_y = PyDict::new(py);
    dim_y.set_item("dims", "y")?;
    dim_y.set_item("data", y)?;

    let dim_bands = PyDict::new(py);
    dim_bands.set_item("dims", "bands")?;
    dim_bands.set_item("data", bands)?;

    // coordinates
    let coords = PyDict::new(py);
    coords.set_item("x", dim_x)?;
    coords.set_item("y", dim_y)?;
    coords.set_item("bands", dim_bands)?;

    // xarray dict
    let dict = PyDict::new(py);
    dict.set_item("data", data)?;
    dict.set_item("dims", dims)?;
    dict.set_item("coords", coords)?;

    let mut result = xarray_module
        .getattr("DataArray")?
        .call_method1("from_dict", (dict,))?;

    if let Some(epsg) = raster_info.epsg {
        result = result.getattr("rio")?.call_method1("write_crs", (epsg,))?;
    };

    Ok(result)
}
