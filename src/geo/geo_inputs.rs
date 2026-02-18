/*
Serialize geopandas geoemetries into WKB for Rust and deserialize into geo::Geometry
This is faster than parsing geometries directly via __geo_interface__
Adapted from https://github.com/geoarrow/geoarrow-rs/blob/main/python/geoarrow-core/src/interop/shapely/from_shapely.rs
 */

use geo_traits::to_geo::ToGeoGeometry;
use geo_types::Geometry;
use pyo3::{
    exceptions::PyValueError,
    intern,
    prelude::*,
    pybacked::PyBackedBytes,
    types::{PyAny, PyDict, PyList, PyBytes},
};
use polars::prelude::*;
use pyo3_polars::PySeries;
use pyo3::Bound;
use wkb::reader::read_wkb;

fn parse_wkb_to_geometry(wkb: &[u8]) -> Option<Geometry<f64>> {
    let wkb_result = read_wkb(wkb).unwrap();
    ToGeoGeometry::try_to_geometry(&wkb_result)
}

fn import_shapely(py: Python) -> PyResult<Bound<PyModule>> {
    let shapely_mod = py.import(intern!(py, "shapely"))?;
    let shapely_version_string = shapely_mod.getattr(intern!(py, "__version__"))?.extract::<String>()?;
    if !shapely_version_string.starts_with('2') {
        Err(PyValueError::new_err("Shapely version 2 required"))
    } else {
        Ok(shapely_mod)
    }
}

fn to_wkb<'a>(py: Python<'a>, shapely_mod: &'a Bound<PyModule>, input: &'a Bound<PyAny>) -> PyResult<Bound<'a, PyAny>> {
    let args = (input,);

    let kwargs = PyDict::new(py);
    kwargs.set_item("output_dimension", 2)?;
    kwargs.set_item("include_srid", false)?;
    kwargs.set_item("flavor", "iso")?;

    shapely_mod.call_method(intern!(py, "to_wkb"), args, Some(&kwargs))
}

pub fn from_shapely(py: Python, input: &Bound<PyAny>) -> PyResult<Vec<Geometry<f64>>> {
    // call `shapely.to_wkb`
    let shapely_mod = import_shapely(py)?;
    let wkb_result = to_wkb(py, &shapely_mod, input)?;

    // build vector of binary geometries
    let mut wkb_output = Vec::with_capacity(wkb_result.len()?);
    for item in wkb_result.try_iter()? {
        // extract bytes and deserialize
        let buf = item?.extract::<PyBackedBytes>()?;
        if let Some(parsed) = parse_wkb_to_geometry(&buf) {
            wkb_output.push(parsed);
        }
    }

    Ok(wkb_output)
}

pub fn from_polars_wkb(input: &Bound<PyAny>) -> PyResult<Vec<Geometry<f64>>> {
    let pyseries: PySeries = input.extract()?;
    let mut wkb_output = Vec::with_capacity(pyseries.0.len());
    for item in pyseries.0.iter() {
            match item {
                AnyValue::Null => continue,  // Skip nulls
                _ => {
                    if let Some(buf) = item.extract_bytes() {
                        let parsed = parse_wkb_to_geometry(buf);  // &[u8]
                        wkb_output.push(parsed);
                    }
                }
            } 
    }
    let wkb_output: Vec<Geometry<f64>> = wkb_output.into_iter().flatten().collect();
    Ok(wkb_output)
}

pub fn from_wkb_list(py_list: &Bound<'_, PyList>) -> PyResult<Vec<Geometry<f64>>> {
    let geometries: Vec<Geometry<f64>> = py_list
        .iter()
        .filter_map(|py_item| {
            let bytes = py_item.downcast::<PyBytes>().ok()?;  
            // parse → Option → ? early None on fail  
            let parsed = parse_wkb_to_geometry(bytes.as_bytes())?;
            Some(parsed)
        })
        .collect();
    Ok(geometries)
}

pub fn from_input(py: Python, input: &Bound<PyAny>) -> PyResult<Vec<Geometry<f64>>> {
    if input.hasattr("geom_type")? {
        from_shapely(py, input)
    } else if let Ok(py_list) = input.downcast::<PyList>() {
        from_wkb_list(py_list)
    } else {
        from_polars_wkb(input)
    }
}
