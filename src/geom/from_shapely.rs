/*
Convert geopandas geoemetries into WKB and pass them to Rust as geoarrow::Table
Then deserializes WKB into geo::Geometry
This is faster than parsing geometries directly via __geo_interface__
Adapted from https://github.com/geoarrow/geoarrow-rs/blob/main/python/geoarrow-core/src/interop/shapely/from_shapely.rs
 */

use geo_types::Geometry;
use geo_traits::to_geo::ToGeoGeometry;
use wkb::reader::read_wkb;
use pyo3::{
    exceptions::PyValueError,
    intern,
    prelude::*,
    types::{PyAny, PyDict},
    pybacked::PyBackedBytes
};

fn parse_wkb_to_geometry(wkb: &[u8]) -> Geometry<f64> {
    let wkb_result = read_wkb(wkb).unwrap();
    ToGeoGeometry::to_geometry(&wkb_result)
}

fn import_shapely(py: Python) -> PyResult<Bound<PyModule>> {
    let shapely_mod = py.import(intern!(py, "shapely"))?;
    let shapely_version_string = shapely_mod
        .getattr(intern!(py, "__version__"))?
        .extract::<String>()?;
    if !shapely_version_string.starts_with('2') {
        Err(PyValueError::new_err("Shapely version 2 required").into())
    } else {
        Ok(shapely_mod)
    }
}

fn to_wkb<'a>(
    py: Python<'a>,
    shapely_mod: &'a Bound<PyModule>,
    input: &'a Bound<PyAny>,
) -> PyResult<Bound<'a, PyAny>> {
    let args = (input,);

    let kwargs = PyDict::new(py);
    kwargs.set_item("output_dimension", 2)?;
    kwargs.set_item("include_srid", false)?;
    kwargs.set_item("flavor", "iso")?;

    Ok(shapely_mod.call_method(intern!(py, "to_wkb"), args, Some(&kwargs))?)
}

pub fn from_shapely(py: Python, input: &Bound<PyAny>) -> PyResult<Vec<Geometry<f64>>> {
    // call `shapely.to_wkb`
    let shapely_mod = import_shapely(py)?;
    let wkb_result = to_wkb(py, &shapely_mod, input)?;
    println!("wkb_result type: {:?}", wkb_result.get_type());
    println!("wkb_result {:?}", wkb_result);
    
    // build vector of binary geometries
    let mut wkb_output = Vec::with_capacity(wkb_result.len()?);
    for item in wkb_result.try_iter()? {
        // extract bytes
        let buf = item?.extract::<PyBackedBytes>()?;
        
        // deserialize
        let parsed = parse_wkb_to_geometry(&buf);        
        wkb_output.push(parsed);
    }
    
    Ok(wkb_output)
}