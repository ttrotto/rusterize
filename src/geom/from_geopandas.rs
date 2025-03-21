/*
Convert geopandas into arrow format and pass it to Rust as geoarrow::Table
This is faster than parsing geometries directly via __geo_interface__
Adapted from https://github.com/geoarrow/geoarrow-rs/blob/main/python/geoarrow-core/src/interop/geopandas/from_geopandas.rs
 */

use arrow::array::BinaryArray;
use geo_types::Geometry;
use geozero::{
    error::GeozeroError,
    wkb::{FromWkb, WkbDialect},
};
use pyo3::{
    exceptions::PyValueError,
    intern,
    prelude::*,
    types::{PyAny, PyDict, PyTuple},
};
use pyo3_arrow::PyTable;
use pyo3_geoarrow::{PyGeoArrowError, PyGeoArrowResult};
use std::io::Cursor;

fn import_geopandas(py: Python) -> PyGeoArrowResult<Bound<PyModule>> {
    let geopandas_mod = py.import(intern!(py, "geopandas"))?;
    let geopandas_version_string = geopandas_mod
        .getattr(intern!(py, "__version__"))?
        .extract::<String>()?;
    let geopandas_major_version = geopandas_version_string
        .split('.')
        .next()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    if geopandas_major_version < 1 {
        Err(PyValueError::new_err("geopandas version 1.0 or higher required").into())
    } else {
        Ok(geopandas_mod)
    }
}

fn parse_wkb_to_geometry(wkb: &[u8]) -> Result<Geometry<f64>, GeozeroError> {
    let mut reader = Cursor::new(wkb);
    FromWkb::from_wkb(&mut reader, WkbDialect::Wkb)
}

pub fn from_geopandas(py: Python, input: &Bound<PyAny>) -> Result<Vec<Geometry>, PyGeoArrowError> {
    let geopandas_mod = import_geopandas(py)?;
    let geodataframe_class = geopandas_mod.getattr(intern!(py, "GeoDataFrame"))?;
    if !input.is_instance(&geodataframe_class)? {
        return Err(PyValueError::new_err(format!(
            "Expected GeoDataFrame input, got {}",
            geodataframe_class
        ))
        .into());
    }

    // convert geopandas to PyTable
    let kwargs = PyDict::new(py);
    kwargs.set_item("geometry_encoding", "wkb")?;
    let table = input
        .call_method(
            intern!(py, "to_arrow"),
            PyTuple::new(py, std::iter::empty::<PyObject>())?,
            Some(&kwargs),
        )?
        .extract::<PyTable>()?;

    // extact inner components
    let (batches, _schema) = table.into_inner();

    // deserialize wkb geometries
    let mut geom_vec = Vec::with_capacity(batches.len());
    for (idx, batch) in batches.iter().enumerate() {
        // convert to BinaryArray
        let geometry_column = batch
            .column(0)
            .as_any()
            .downcast_ref::<BinaryArray>()
            .ok_or(PyGeoArrowError::from(PyValueError::new_err(format!(
                "Unable to downcast geometries to arrow::BinaryArray at index {}",
                idx
            ))))?;

        // collect
        geom_vec.extend(
            geometry_column
                .iter()
                .filter_map(|wkb| wkb.and_then(|wkb| parse_wkb_to_geometry(wkb).ok())),
        );
    }
    Ok(geom_vec)
}
