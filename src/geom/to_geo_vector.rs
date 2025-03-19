/*
Extract geometries from geoarrow::Table
*/

use crate::geom::from_geopandas::from_geopandas;
use geo_traits::to_geo::ToGeoGeometryCollection;
use geo_types::Geometry;
use geoarrow::array::GeometryCollectionArray;
use geoarrow::trait_::ArrayAccessor;
use pyo3::exceptions::PyValueError;
use pyo3::{Bound, PyAny, Python};

use pyo3_geoarrow::PyGeoArrowError;

pub fn to_geo_vector(py: Python, input: &Bound<PyAny>) -> Result<Vec<Geometry>, PyGeoArrowError> {
    // serialize geometries from geopandas
    let table = from_geopandas(py, input)?;

    // collect geometries
    let mut geovec: Vec<Geometry> = Vec::new();

    // Table -> ChunkedArray -> GeometryCollectionArray
    let chunked_array = table
        .geometry_column(None)
        .expect("No geometry column found!");

    let geom_array = chunked_array
        .as_ref()
        .as_any()
        .downcast_ref::<GeometryCollectionArray>()
        .ok_or(PyGeoArrowError::from(PyValueError::new_err(
            "Can't downcast geometry column, expected a GeometryCollection",
        )))?;

    // push geometries out of the collection into a unified vector
    geovec.extend(
        geom_array
            .iter()
            .filter_map(|geom_collection| {
                geom_collection
                    .and_then(|gc| ToGeoGeometryCollection::try_to_geometry_collection(&gc))
            })
            .flat_map(|gc| gc.0),
    );
    Ok(geovec)
}
