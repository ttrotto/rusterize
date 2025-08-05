/*
Build dictionary for xarray construction.
 */
use crate::structs::raster::RasterInfo;
use ndarray::Array3;
use num_traits::Num;
use numpy::{Element, IntoPyArray};
use pyo3::{
    prelude::*,
    types::{PyDict, PyList},
};

pub fn build_xarray<T>(
    py: Python,
    raster_info: RasterInfo,
    ret: Array3<T>,
    band_names: Vec<String>,
) -> PyResult<Bound<PyDict>>
where
    T: Num + Element,
{
    let data = ret.into_pyarray(py);
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

    // xarray
    let xarray = PyDict::new(py);
    xarray.set_item("data", data)?;
    xarray.set_item("dims", dims)?;
    xarray.set_item("coords", coords)?;
    Ok(xarray)
}
