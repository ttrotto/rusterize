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
    geometry: Vec<Geometry>,
    ras_info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    field: Option<&'r Float64Chunked>,
    by: Option<&'r StringChunked>,
}

impl Rusterize<'_> {
    fn new(&self,
           mut geometry: Vec<Geometry>,
           ras_info: Raster,
           pixel_fn: PixelFn,
           background: f64,
           mut df: Option<DataFrame>,
           field_name: Option<&str>,
           by_name: Option<&str>,
    ) -> Self {
        // check if any bad geometry
        let good_geom: Vec<bool> = geometry
            .iter()
            .map(|geom| matches!(geom, Geometry::Polygon(_) | Geometry::MultiPolygon(_)))
            .collect();

        // retain only good geometries
        if good_geom.iter().any(|&valid| !valid)  {
            println!("Detected unsupported geometries, will be removed.");
            let mut iter = good_geom.iter();
            geometry.retain(|_| *iter.next().unwrap());
            df = df.and_then(|inner| {
                inner
                    .filter(&BooleanChunked::from_iter_values(
                        PlSmallStr::from("good_geom"),
                        good_geom.into_iter(),
                    ))
                    .ok()
            });
        }

        // extract field and by
        let (field, by): (Option<&Float64Chunked>, Option<&StringChunked>) = match df {
            None => {
                // case 1: no DataFrame, return dummy field and None for `by`
                (
                    Some(self.make_dummy_field()),
                    None,
                )
            }
            Some(df) => {
                // casting and extraction
                match (field_name, by_name) {
                    (Some(field_name), Some(by_name)) => {
                        // case 2: both `field` and `by` are present
                        match (df.schema().get(field_name), df.schema().get(by_name)) {
                            // correct dtype
                            (Some(&DataType::Float64), Some(&DataType::String)) => (
                                Some(df.column(field_name).unwrap().f64().unwrap()),
                                Some(df.column(by_name).unwrap().str().unwrap()),
                            ),
                            // needs casting
                            _ => {
                                let casted = df
                                    .lazy()
                                    .select([
                                        col(field_name).cast(DataType::Float64),
                                        col(by_name).cast(DataType::String),
                                    ])
                                    .collect()
                                    .unwrap();
                                (
                                    Some(casted.column(field_name).unwrap().f64().unwrap()),
                                    Some(casted.column(by_name).unwrap().str().unwrap()),
                                )
                            }
                        }
                    }
                    (Some(field_name), None) => {
                        // case 3: only `field` is present
                        match df.schema().get(field_name) {
                            // correct type
                            Some(&DataType::Float64) => {
                                (Some(df.column(field_name).unwrap().f64().unwrap()), None)
                            }
                            // needs casting
                            _ => {
                                let casted = df
                                    .lazy()
                                    .select([col(field_name).cast(DataType::Float64)])
                                    .collect()
                                    .unwrap();
                                (
                                    Some(casted.column(field_name).unwrap().f64().unwrap()),
                                    None,
                                )
                            }
                        }
                    }
                    (None, Some(by_name)) => {
                        // case 4: only `by` is present
                        match df.schema().get(by_name) {
                            // correct type
                            Some(&DataType::String) => (
                                Some(self.make_dummy_field()),
                                Some(df.column(by_name).unwrap().str().unwrap()),
                            ),
                            // needs casting
                            _ => {
                                let casted = df
                                    .lazy()
                                    .select([col(by_name).cast(DataType::String)])
                                    .collect()
                                    .unwrap();
                                (
                                    Some(self.make_dummy_field()),
                                    Some(casted.column(by_name).unwrap().str().unwrap()),
                                )
                            }
                        }
                    }
                    (None, None) => {
                        // neither `field` nor `by` is specified, invalid case
                        panic!("Both `field` and `by` cannot be None with a DataFrame present.");
                    }
                }
            }
        };

        Self {
            geometry,
            ras_info,
            pixel_fn,
            background,
            field,
            by,
        }
    }

    fn make_dummy_field(&self) -> &Float64Chunked {
        &Float64Chunked::from_vec(PlSmallStr::from("dummy"), vec![1.0; self.geometry.len()])
    }

    fn rusterize_sequential(self) -> Array3<f64> {
        let vfield: Option<Vec<f64>> = self.field.and_then(|inner| inner.into_iter().collect());
        let vby: Option<Vec<String>> = self.by.and_then(|inner| inner.into_iter().collect());

        match vby {
            Some(vby) => {}
            None => {
                // singleband raster
                let mut raster = self.ras_info.build_raster(1);

                vfield.into_iter().zip(self.geometry.into_iter()).for_each(
                    |(field_value, geom)| match field_value {
                        Some(field_value) => {
                            // process only non-empty field values
                            rasterize_polygon(
                                &self.ras_info,
                                geom,
                                &field_value,
                                &mut raster.index_axis_mut(Axis(0), 0),
                                &self.pixel_fn,
                            )
                        }
                        None => {}
                    },
                );

                // return
                raster
            }
        }
    }
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pygeometry: &PyAny,
    pyinfo: &PyAny,
    pypixel_fn: &PyString,
    pybackground: &PyAny,
    pydf: Option<PyDataFrame>,
    pyfield: Option<&PyString>,
    pyby: Option<&PyString>,
) -> PyResult<&'py PyArray3<f64>> {
    // extract dataframe
    let df: Option<DataFrame> = pydf.and_then(|inner| Some(inner.into()));

    // extract
    let geometry: Vec<Geometry> = pygeometry.as_geometry_vec()?.0;

    // extract raster information
    let raster_info = Raster::from(&pyinfo);

    // extract function arguments
    let f = pypixel_fn.to_str()?;
    let pixel_fn = set_pixel_function(f);
    let background = pybackground.extract::<f64>()?;
    let field = pyfield.and_then(|inner| Some(inner.to_str().unwrap()));
    let by = pyby.and_then(|inner| Some(inner.to_str().unwrap()));

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
