#![feature(extract_if)]

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
        {Array, Array3, Axis},
    },
    PyArray1, PyArray3, ToPyArray,
};
use polars::prelude::*;
use py_geo_interface::from_py::AsGeometryVec;
use pyo3::{
    prelude::*,
    types::{PyAny, PyList, PyString},
};
use pyo3_polars::PyDataFrame;
use std::sync::mpsc::channel;
use structs::raster::RasterInfo;

type TupleToPython<'py> = PyResult<(
    Bound<'py, PyArray3<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyList>,
)>;

fn rusterize_rust(
    mut geometry: Vec<Geometry>,
    raster_info: RasterInfo,
    pixel_fn: PixelFn,
    background: f64,
    threads: usize,
    mut df: Option<DataFrame>,
    field_name: Option<&str>,
    by_name: Option<&str>,
) -> (Array3<f64>, Vec<String>) {
    // check if any bad geometry
    let good_geom: Vec<bool> = geometry
        .iter()
        .map(|geom| matches!(geom, Geometry::Polygon(_) | Geometry::MultiPolygon(_)))
        .collect();

    // retain only good geometries
    if good_geom.iter().any(|&valid| !valid) {
        println!("Detected unsupported geometries, will be dropped.");
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
    let mut raster: Array3<f64>;
    let mut band_names: Vec<String> = vec![String::from("band1")];
    match by {
        Some(by) => {
            // open channel for sending and receiving band names
            let (sender, receiver) = channel();

            // multiband raster on by groups
            let groups = by.group_tuples(true, false).expect("No groups found!");
            raster = raster_info.build_raster(groups.len());

            // parallel iterator along bands, zipped with the corresponding groups
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            pool.install(|| {
                raster
                    .outer_iter_mut()
                    .into_par_iter()
                    .zip(groups.into_idx().into_par_iter())
                    .for_each_with(sender, |sendr, (mut band, (group_idx, idxs))| {
                        // send band names to receiver
                        if let Some(name) = by.get(group_idx as usize) {
                            sendr.send(name.to_string()).unwrap()
                        }

                        // rasterize polygons
                        for &i in idxs.iter() {
                            if let (Some(fv), Some(geom)) =
                                (field.get(i as usize), geometry.get(i as usize))
                            {
                                rasterize_polygon(&raster_info, geom, &fv, &mut band, &pixel_fn);
                            }
                        }
                    })
            });
            // collect band names
            band_names = receiver.iter().collect();
        }
        None => {
            // singleband raster
            raster = raster_info.build_raster(1);

            // rasterize polygons
            field
                .into_iter()
                .zip(geometry)
                .for_each(|(field_value, geom)| {
                    if let Some(fv) = field_value {
                        // process only non-empty field values
                        rasterize_polygon(
                            &raster_info,
                            &geom,
                            &fv,
                            &mut raster.index_axis_mut(Axis(0), 0),
                            &pixel_fn,
                        )
                    }
                });
        }
    }

    // replace NaN with background
    if !background.is_nan() {
        raster.mapv_inplace(|x| if x.is_nan() { background } else { x })
    };
    (raster, band_names)
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pygeometry: &Bound<'py, PyAny>,
    pyinfo: &Bound<'py, PyAny>,
    pypixel_fn: &Bound<'py, PyString>,
    pythreads: &Bound<'py, PyAny>,
    pydf: Option<PyDataFrame>,
    pyfield: Option<&Bound<'py, PyString>>,
    pyby: Option<&Bound<'py, PyString>>,
    pybackground: Option<&Bound<'py, PyAny>>,
) -> TupleToPython<'py> {
    // get number of threads
    let op_threads = pythreads.extract::<isize>()?;
    let threads = if op_threads <= 0 {
        std::thread::available_parallelism()?.get()
    } else {
        op_threads as usize
    };

    // extract dataframe
    let df = pydf.map(|inner| inner.into());

    // extract geometries
    let geometry = pygeometry.as_geometry_vec()?;

    // extract raster information
    let raster_info = RasterInfo::from(pyinfo);

    // extract function arguments
    let f = pypixel_fn.to_str()?;
    let pixel_fn = set_pixel_function(f);
    let background = pybackground
        .and_then(|inner| inner.extract::<f64>().ok())
        .unwrap_or(f64::NAN);
    let field = pyfield.map(|inner| inner.to_str().unwrap());
    let by = pyby.map(|inner| inner.to_str().unwrap());

    // construct coordinates
    let x_coords = Array::range(
        raster_info.xmin,
        raster_info.xmin + raster_info.ncols as f64 * raster_info.xres,
        raster_info.xres,
    )
    .to_pyarray_bound(py);
    let y_coords = Array::range(
        raster_info.ymax,
        raster_info.ymax - raster_info.nrows as f64 * raster_info.yres,
        -raster_info.yres,
    )
    .to_pyarray_bound(py);

    // rusterize
    let (ret, band_names) = rusterize_rust(
        geometry,
        raster_info,
        pixel_fn,
        background,
        threads,
        df,
        field,
        by,
    );
    let pyret = ret.to_pyarray_bound(py);
    let pynames = PyList::new_bound(py, band_names);
    Ok((pyret, x_coords, y_coords, pynames))
}

#[pymodule]
fn rusterize(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
