/*
Serialize geopandas geoemetries into WKB for Rust and deserialize into geo_types::Geometry
This is faster than parsing geometries directly via __geo_interface__
 */

use geo_traits::to_geo::ToGeoGeometry;
use geo_types::Geometry;
use polars::{datatypes::DataType, error::PolarsError, prelude::*};
use pyo3::{
    Bound,
    exceptions::{PyTypeError, PyValueError},
    intern,
    prelude::*,
    pybacked::PyBackedBytes,
    types::{PyAny, PyBytes, PyDict, PyList, PyString},
};
use pyo3_polars::PySeries;
use rayon::iter::ParallelIterator;
use std::ops::Deref;
use wkb::reader::read_wkb;
use wkt::TryFromWkt;

pub struct ParsedGeometry(pub Vec<Geometry<f64>>);

impl ParsedGeometry {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, index: usize) -> Option<&Geometry<f64>> {
        self.0.get(index)
    }
}

impl<'a> IntoIterator for &'a ParsedGeometry {
    type Item = &'a Geometry<f64>;
    type IntoIter = std::slice::Iter<'a, Geometry<f64>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Deref for ParsedGeometry {
    type Target = [Geometry<f64>];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl FromPyObject<'_, '_> for ParsedGeometry {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        // geopandas.GeoDataFrame
        if obj.hasattr("geom_type")? {
            let py = obj.py();

            // shapely >= 2.0.0
            let shapely_mod = py.import(intern!(py, "shapely"))?;
            let shapely_version_string = shapely_mod.getattr(intern!(py, "__version__"))?.extract::<String>()?;
            if !shapely_version_string.starts_with('2') {
                return Err(PyValueError::new_err("Shapely version 2 required"));
            }

            let wkb_result = to_wkb(py, &shapely_mod, &obj)?;

            return parse_iterable_wkb(&wkb_result);
        }

        if obj.is_instance_of::<PyList>() || obj.get_type().name()? == "ndarray" {
            if obj.is_empty()? {
                return Err(PyValueError::new_err("No geometries found."));
            }

            // check first item to determine parsing strategy
            let first = obj.get_item(0)?;
            if first.is_instance_of::<PyBytes>() {
                return parse_iterable_wkb(&obj);
            } else if first.is_instance_of::<PyString>() {
                return parse_iterable_wkt(&obj);
            } else {
                return Err(PyValueError::new_err(
                    "Iterable must contain geometries as bytes (WKB) or string (WKT).",
                ));
            }
        }

        if let Ok(pyseries) = obj.extract::<PySeries>() {
            let series: Series = pyseries.into();
            return parse_polars_series(series).map_err(|e| PyTypeError::new_err(e.to_string()));
        }

        Err(PyTypeError::new_err("Unsupported geometry input type."))
    }
}

fn try_parse_wkb_to_geometry(wkb: &[u8]) -> Option<Geometry<f64>> {
    let wkb_result = read_wkb(wkb).expect(
        "Cannot parse geometry. Check that the WKB bytes are valid. \
       This may happen when you convert a list of WKB stored as python 'object' into a numpy array.",
    );
    ToGeoGeometry::try_to_geometry(&wkb_result)
}

fn try_parse_wkt_to_geometry(wkt: &str) -> Option<Geometry<f64>> {
    Some(Geometry::try_from_wkt_str(wkt).unwrap())
}

fn to_wkb<'a>(
    py: Python<'a>,
    shapely_mod: &'a Bound<PyModule>,
    input: &Bound<'a, PyAny>,
) -> PyResult<Bound<'a, PyAny>> {
    let args = (input,);

    let kwargs = PyDict::new(py);
    kwargs.set_item("output_dimension", 2)?;
    kwargs.set_item("include_srid", false)?;
    kwargs.set_item("flavor", "iso")?;

    shapely_mod.call_method(intern!(py, "to_wkb"), args, Some(&kwargs))
}

fn parse_iterable_wkb(input: &Bound<PyAny>) -> PyResult<ParsedGeometry> {
    let mut geoms = Vec::with_capacity(input.len()?);
    for item in input.try_iter()? {
        let buf = item?.extract::<PyBackedBytes>()?;
        if let Some(parsed) = try_parse_wkb_to_geometry(&buf) {
            geoms.push(parsed);
        }
    }

    if geoms.is_empty() {
        return Err(PyValueError::new_err(
            "Could not parse geometry. Only WKT or WKB formats are supported.",
        ));
    }

    Ok(ParsedGeometry(geoms))
}

fn parse_iterable_wkt(input: &Bound<'_, PyAny>) -> PyResult<ParsedGeometry> {
    let mut geoms = Vec::with_capacity(input.len().unwrap_or(0));
    for item in input.try_iter()? {
        let s = item?.extract::<String>()?;
        if let Some(parsed) = try_parse_wkt_to_geometry(&s) {
            geoms.push(parsed);
        }
    }

    if geoms.is_empty() {
        return Err(PyValueError::new_err(
            "Could not parse geometry. Only WKT or WKB formats are supported.",
        ));
    }

    Ok(ParsedGeometry(geoms))
}

fn parse_polars_series(input: Series) -> Result<ParsedGeometry, PolarsError> {
    let wkb_output = match input.dtype() {
        DataType::Binary => input
            .binary()?
            .iter()
            .filter_map(|item| item.and_then(try_parse_wkb_to_geometry))
            .collect(),
        DataType::String => input
            .str()?
            .par_iter()
            .filter_map(|item| item.and_then(try_parse_wkt_to_geometry))
            .collect(),
        _ => unimplemented!("Unsupported dtype for geometry column"),
    };
    Ok(ParsedGeometry(wkb_output))
}
