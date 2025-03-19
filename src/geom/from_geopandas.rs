/*
Convert geopandas into arrow format and pass it to Rust as geoarrow::Table
This is much faster than parsing geometries directly via __geo_interface__
Adapted from https://github.com/geoarrow/geoarrow-rs/blob/main/python/geoarrow-core/src/interop/geopandas/from_geopandas.rs
 */
use geoarrow::array::CoordType;
use geoarrow::datatypes::{Dimension, NativeType};
use geoarrow::error::GeoArrowError;
use geoarrow::table::Table;
use pyo3::exceptions::PyValueError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3::PyAny;
use pyo3_arrow::PyTable;
use pyo3_geoarrow::{PyGeoArrowError, PyGeoArrowResult};

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

fn pytable_to_table(table: PyTable) -> Result<Table, GeoArrowError> {
    let (batches, schema) = table.into_inner();
    Table::try_new(batches, schema)
}

pub fn from_geopandas(py: Python, input: &Bound<PyAny>) -> Result<Table, PyGeoArrowError> {
    let geopandas_mod = import_geopandas(py)?;
    let geodataframe_class = geopandas_mod.getattr(intern!(py, "GeoDataFrame"))?;
    if !input.is_instance(&geodataframe_class)? {
        return Err(PyValueError::new_err("Expected GeoDataFrame input.").into());
    }
    let kwargs = PyDict::new(py);
    kwargs
        .set_item("geometry_encoding", "wkb")
        .expect("Can't set dictionary keywords");
    let table = input
        .call_method(
            intern!(py, "to_arrow"),
            PyTuple::new(py, std::iter::empty::<PyObject>())?,
            Some(&kwargs),
        )?
        .extract::<PyTable>()?;
    println!("table: {:?}", table);    
    let table = pytable_to_table(table).unwrap();
    println!("here1");
    let table = table
        .parse_serialized_geometry(
            table.default_geometry_column_idx().unwrap(),
            Some(NativeType::GeometryCollection(CoordType::Interleaved, Dimension::XY)),
        )
        .expect("Can't deserialize geometries from Arrow Table into GeometryCollection");
    println!("here2");
    Ok(table)
}
