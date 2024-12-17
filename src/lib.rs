#![feature(extract_if)]
extern crate blas_src;

mod structs {
    pub mod edge;
    pub mod raster;
}
mod edgelist;
mod pixel_functions;
mod rasterize_polygon;

use crate::pixel_functions::{set_pixel_function, PixelFn};
use crate::rasterize_polygon::rasterize_polygon;
use geo_types::Geometry;
use gxhash::{HashSet, HashSetExt};
use numpy::{
    ndarray::{Array3, Axis},
    PyArray3, ToPyArray,
};
use polars::prelude::*;
use py_geo_interface::wrappers::f64::AsGeometryVec;
use pyo3::{
    prelude::*,
    types::{PyAny, PyString},
};
use pyo3_polars::PyDataFrame;
use structs::raster::Raster;

struct Rusterize<'r> {
    df: DataFrame,
    geometry: Vec<Geometry>,
    ras_info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    vfield: &'r Float64Chunked,
    vby: Option<Vec<String>>,
}

impl Rusterize {
    fn new(
        mut df: DataFrame,
        mut geometry: Vec<Geometry>,
        ras_info: Raster,
        pixel_fn: PixelFn,
        background: f64,
        field: String,
        by: String,
    ) -> Self {
        // build default geometry hasher
        let mut geomset_default: HashSet<&str> = HashSet::with_capacity(2);
        geomset_default.insert("Polygon");
        geomset_default.insert("MultiPolygon");

        // pass geometry to hashset and check it is subset of default
        let mut geomset: HashSet<&str> = HashSet::new();
        let good_geom: Vec<bool> = geometry
            .iter()
            .enumerate()
            .map(|(i, geom)| match geom {
                &Geometry::Polygon(_) => {
                    geomset.insert("Polygon");
                    true
                },
                &Geometry::MultiPolygon(_) => {
                    geomset.insert("Polygon");
                    true
                },
                _ => {
                    geomset.insert("Bad");
                    false
                }
            })
            .collect();

        // filter dataframe if bad geometries
        if !geomset.is_subset(&geomset_default) {
            println!("Detected unsupported geometries, will be removed");
            // keep good geometries based on row index
            df = df.lazy()
                .with_row_index(PlSmallStr::from("idx"), None)
                .filter(col("idx").is_in(good_geom))
                .collect()
                .unwrap();
            let mut iter = good_geom.iter();
            geometry.retain(|_| *iter.next().unwrap());
        }

        // extract field and by
        let (vfield, vby): (&Float64Chunked, Option<Vec<String>>);
        if field.is_empty() {
            // by also empty
            (vfield, vby) = (
                Series::new(PlSmallStr::from("field_f64"), vec![1; df.height()])
                    .f64()
                    .unwrap(),
                None,
            )
        }

        // casting and extraction
        let field_name = field.as_str();
        let (vfield, vby) = if !by.is_empty() {
            // handle field and by
            let by_name = by.as_str();
            match (df.schema().get(field_name), df.schema().get(by_name)) {
                // correct dtype
                (Some(&DataType::Float64), Some(&DataType::String)) => (
                    df.column(field_name).unwrap().f64().unwrap(),
                    df.column(by_name)
                        .unwrap()
                        .str()
                        .unwrap()
                        .into_iter()
                        .collect(),
                ),
                _ => {
                    // needs casting
                    let casted = df
                        .lazy()
                        .select([
                            col(field_name).cast(DataType::Float64),
                            col(by_name).cast(DataType::String),
                        ])?
                        .collect();
                    (
                        casted.column(field_name)?.f64().unwrap(),
                        casted
                            .column(by_name)
                            .unwrap()
                            .str()
                            .unwrap()
                            .into_iter()
                            .collect(),
                    )
                }
            };
        } else {
            // handle only field
            match df.schema().get(field_name) {
                // correct type
                Some(DataType::Float64) => (df.column(field_name).unwrap().f64().unwrap(), None),
                // needs casting
                _ => {
                    let casted = df
                        .lazy()
                        .select([col(field_name).cast(DataType::Float64)])
                        .collect()
                        .unwrap();
                    (casted.column(field_name).unwrap().f64().unwrap(), None)
                }
            }
        };
        Self {
            df,
            geometry,
            ras_info,
            pixel_fn,
            background,
            vfield,
            vby,
        }
    }

    fn rusterize_sequential(self) -> Array3<f64> {
        if Some(self.vby) {
            // multiband raster
            let bands
        } else {
            // singleband raster
            let mut raster = self.ras_info.build_raster(1);

            // iter rasterize
            self.vfield
                .into_iter()
                .zip(self.geometry.into_iter())
                .for_each(|(field_value, geom)| match field_value {
                    Some(field_value) => {
                        // process only non-empty field values
                        rasterize_polygon(
                            &self.ras_info,
                            &geom,
                            &field_value,
                            &mut raster.index_axis_mut(Axis(0), 0),
                            &self.pixel_fn,
                        )
                    }
                    None => {}
                });

            // return
            raster
        }
    }
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pydf: PyDataFrame,
    pygeometry: &PyAny,
    pyinfo: &PyAny,
    pypixel_fn: &PyString,
    pybackground: &PyAny,
    pyfield: Option<&PyString>,
    pyby: Option<&PyString>,
) -> PyResult<&'py PyArray3<f64>> {
    // extract dataframe
    let mut df: DataFrame = pydf.into();

    // extract geometries
    let mut geometry = pygeometry.as_geometry_vec()?.0;

    // extract raster information
    let raster_info = Raster::from(&pyinfo);

    // extract function arguments
    let fun = pypixel_fn.to_str()?;
    let pixel_fn = set_pixel_function(fun);
    let background = pybackground.extract::<f64>()?;
    let field = match pyfield {
        Some(inner) => inner.to_string(),
        None => String::new(),
    };
    let by = match pyby {
        Some(inner) => inner.to_string(),
        None => String::new(),
    };

    // rusterize
    let ret = py.allow_threads(|| {
        let rclass = Rusterize::new(df, geometry, raster_info, pixel_fn, background, field, by);
        rclass.rusterize_sequential();
    });
    Ok(ret.to_pyarray(py))
}

fn rusterize(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
