use geo_traits::to_geo::ToGeoGeometry;
use geo_types::Geometry;
use polars::{datatypes::DataType, prelude::*};
use pyo3::{
    Bound,
    exceptions::{PyTypeError, PyValueError},
    intern,
    prelude::*,
    pybacked::{PyBackedBytes, PyBackedStr},
    types::{PyAny, PyBytes, PyDict, PyList, PyString},
};
use pyo3_polars::PySeries;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusterize::prelude::{RusterizeError, RusterizeResult};
use wkb::reader::read_wkb;
use wkt::TryFromWkt;

const WKB_ERR: &str = "Cannot parse geometry. Check that WKB is valid. \
    This may happen when you convert a list of WKB stored as python 'object' into a numpy array.";
const WKT_ERR: &str = "Cannot parse geometry. Check that the WKT is valid.";

macro_rules! bail_if_empty_geoms {
    ($identity:ident) => {
        if $identity.is_empty() {
            return Err(PyValueError::new_err(
                "Could not parse geometry. Only WKT or WKB formats are supported.",
            ));
        }
    };
}

pub(crate) struct ParsedGeometry(Vec<Geometry<f64>>);

impl AsRef<[Geometry<f64>]> for ParsedGeometry {
    fn as_ref(&self) -> &[Geometry<f64>] {
        self.0.as_slice()
    }
}

impl FromPyObject<'_, '_> for ParsedGeometry {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        if obj.is_instance_of::<PyList>() || obj.get_type().name()? == "ndarray" {
            if obj.is_empty()? {
                return Err(PyValueError::new_err("No geometries found."));
            }

            // check first item to determine parsing strategy
            let first = obj.get_item(0)?;
            if first.is_instance_of::<PyBytes>() {
                return parse_sequence_wkb(&obj);
            } else if first.is_instance_of::<PyString>() {
                return parse_sequence_wkt(&obj);
            } else if first.hasattr("geom_type")? {
                // list of shapely geometries
                let wkb_result = to_wkb(&obj)?;
                return parse_sequence_wkb(&wkb_result);
            } else {
                return Err(PyValueError::new_err(
                    "Sequence must contain geometries as shapely Geometry, bytes (WKB), or string (WKT).",
                ));
            }
        }

        if let Ok(pyseries) = obj.extract::<PySeries>() {
            let series: Series = pyseries.into();
            return obj
                .py()
                .detach(|| parse_polars_series(series))
                .map_err(|e| PyTypeError::new_err(e.to_string()));
        }

        Err(PyTypeError::new_err("Unsupported geometry input type."))
    }
}

/// Parse `items` in parallel, `None` if any fails.
/// Collecting into `Vec<Option<_>>` lets rayon write in place; collecting into `Option<Vec<_>>`
/// directly gathers chunks in a linked list and copies every geometry again.
fn par_parse<T: Sync>(
    items: &[T],
    parse: impl Fn(&T) -> Option<Geometry<f64>> + Sync + Send,
) -> Option<Vec<Geometry<f64>>> {
    items.par_iter().map(parse).collect::<Vec<_>>().into_iter().collect()
}

fn try_parse_wkb_to_geometry(wkb: &[u8]) -> Option<Geometry<f64>> {
    read_wkb(wkb).ok()?.try_to_geometry()
}

fn try_parse_wkt_to_geometry(wkt: &str) -> Option<Geometry<f64>> {
    Geometry::try_from_wkt_str(wkt).ok()
}

fn to_wkb<'a>(input: &Bound<'a, PyAny>) -> PyResult<Bound<'a, PyAny>> {
    let py = input.py();

    // shapely >= 2.0.0
    let shapely_mod = py.import(intern!(py, "shapely"))?;
    let shapely_version_string = shapely_mod.getattr(intern!(py, "__version__"))?.extract::<String>()?;
    if !shapely_version_string.starts_with('2') {
        return Err(PyValueError::new_err("Shapely version 2 required"));
    }

    let args = (input,);
    let kwargs = PyDict::new(py);
    kwargs.set_item("output_dimension", 2)?;
    kwargs.set_item("include_srid", false)?;
    kwargs.set_item("flavor", "iso")?;

    shapely_mod.call_method(intern!(py, "to_wkb"), args, Some(&kwargs))
}

fn parse_sequence_wkb(input: &Bound<PyAny>) -> PyResult<ParsedGeometry> {
    // borrow every buffer while holding the GIL, then parse in parallel without it
    let bufs = input
        .try_iter()?
        .map(|item| Ok(item?.extract::<PyBackedBytes>()?))
        .collect::<PyResult<Vec<PyBackedBytes>>>()?;
    let geoms = input
        .py()
        .detach(|| par_parse(&bufs, |buf| try_parse_wkb_to_geometry(buf)))
        .ok_or_else(|| PyValueError::new_err(WKB_ERR))?;

    bail_if_empty_geoms!(geoms);
    Ok(ParsedGeometry(geoms))
}

fn parse_sequence_wkt(input: &Bound<'_, PyAny>) -> PyResult<ParsedGeometry> {
    // borrow every string while holding the GIL, then parse in parallel without it
    let strs = input
        .try_iter()?
        .map(|item| Ok(item?.extract::<PyBackedStr>()?))
        .collect::<PyResult<Vec<PyBackedStr>>>()?;
    let geoms = input
        .py()
        .detach(|| par_parse(&strs, |s| try_parse_wkt_to_geometry(s)))
        .ok_or_else(|| PyValueError::new_err(WKT_ERR))?;

    bail_if_empty_geoms!(geoms);
    Ok(ParsedGeometry(geoms))
}

fn parse_polars_series(input: Series) -> RusterizeResult<ParsedGeometry> {
    let wkb_output = match input.dtype() {
        DataType::Binary => par_parse(&input.binary()?.iter().collect::<Vec<_>>(), |item| {
            item.and_then(try_parse_wkb_to_geometry)
        })
        .ok_or_else(|| RusterizeError::ValueError(WKB_ERR))?,
        DataType::String => par_parse(&input.str()?.iter().collect::<Vec<_>>(), |item| {
            item.and_then(try_parse_wkt_to_geometry)
        })
        .ok_or_else(|| RusterizeError::ValueError(WKT_ERR))?,
        _ => return Err(RusterizeError::ValueError("Unsupported dtype for geometry column")),
    };
    Ok(ParsedGeometry(wkb_output))
}
