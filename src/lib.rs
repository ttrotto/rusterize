#![feature(extract_if)]
extern crate blas_src;

mod allocator;
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
    ndarray::{
        parallel::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator},
        {Array3, ArrayViewMut2, Axis},
    },
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

fn rusterize_rust(
    mut geometry: Vec<Geometry>,
    ras_info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    mut df: Option<DataFrame>,
    field_name: Option<&str>,
    by_name: Option<&str>,
) -> Array3<f64> {
    // check if any bad geometry
    let good_geom: Vec<bool> = geometry
        .iter()
        .map(|geom| matches!(geom, Geometry::Polygon(_) | Geometry::MultiPolygon(_)))
        .collect();

    // retain only good geometries
    if good_geom.iter().any(|&valid| !valid) {
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

    // extract `field` and `by`
    let casted: DataFrame;
    let (field, by) = match df {
        None => (
            // case 1: create a dummy `field`
            &Float64Chunked::from_vec(PlSmallStr::from("field_f64"), vec![1.0; geometry.len()]),
            None,
        ),
        Some(df) => {
            let mut lf = df.lazy();
            match (field_name, by_name) {
                (Some(field_name), Some(by_name)) => {
                    // case 2: both `field` and `by` specified
                    lf = lf.with_columns([
                        col(field_name).cast(DataType::Float64).alias("field_f64"),
                        col(by_name).cast(DataType::String).alias("by_str"),
                    ]);
                }
                (Some(field_name), None) => {
                    // case 3: only `field` specified
                    lf = lf.with_column(col(field_name).cast(DataType::Float64).alias("field_f64"));
                }
                (None, Some(by_name)) => {
                    // case 4: only `by` specified
                    lf = lf.with_columns([
                        lit(1.0).alias("field_f64"), // dummy `field`
                        col(by_name).cast(DataType::String).alias("by_str"),
                    ]);
                }
                (None, None) => {
                    // case 5: neither `field` nor `by` specified
                    panic!("Either `field` or `by` must be specified.");
                }
            }

            // collect the result
            casted = lf.collect().unwrap();

            (
                casted.column("field_f64").unwrap().f64().unwrap(),
                casted.column("by_str").ok().and_then(|col| col.str().ok()),
            )
        }
    };

    // main
    match by {
        Some(by) => {
            // multiband raster
            let groups = by.group_tuples(true, true).unwrap();
            let mut raster = ras_info.build_raster(groups.len());

            // parallel iterator along bands, zipped with the corresponding group
            raster
                .outer_iter_mut()
                .into_par_iter()
                .zip(groups.into_idx().into_par_iter())
                .for_each(|(mut band, (_, idxs))| {
                    // group field values and geometries
                    let pairs: Vec<(Option<f64>, &Geometry)> = idxs
                        .iter()
                        .map(|&i| (field.get(i as usize), &geometry[i as usize]))
                        .collect();

                    // call function
                    pairs.into_iter().for_each(|(field_value, geom)| {
                        if let Some(fv) = field_value {
                            // only non-empty field values
                            rasterize_polygon(&ras_info, geom, &fv, &mut band, &pixel_fn);
                        }
                    });
                });
            raster
        }
        None => {
            // singleband raster
            let mut raster = ras_info.build_raster(1);

            // call function
            field
                .into_iter()
                .zip(geometry.into_iter())
                .for_each(|(field_value, geom)| {
                    if let Some(fv) = field_value {
                        // process only non-empty field values
                        rasterize_polygon(
                            &ras_info,
                            &geom,
                            &fv,
                            &mut raster.index_axis_mut(Axis(0), 0),
                            &pixel_fn,
                        )
                    }
                });
            raster
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
    let ret = rusterize_rust(geometry, raster_info, pixel_fn, background, df, field, by);
    Ok(ret.to_pyarray(py))
}

#[pymodule]
fn rusterize(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
