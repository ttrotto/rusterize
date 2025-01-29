#![feature(extract_if)]

mod allocator;
mod structs {
    pub mod edge;
    pub mod raster;
    pub mod xarray;
}
mod edgelist;
mod geo_validate;
mod pixel_functions;
mod rasterize_polygon;

use crate::geo_validate::validate_geometries;
use crate::pixel_functions::{set_pixel_function, PixelFn};
use crate::rasterize_polygon::rasterize_polygon;
use geo_types::Geometry;
use numpy::{
    ndarray::{
        parallel::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator},
        {Array3, Axis},
    },
    IntoPyArray,
};
use polars::prelude::*;
use py_geo_interface::from_py::AsGeometryVec;
use pyo3::{
    prelude::*,
    types::{PyAny, PyList},
};
use pyo3_polars::PyDataFrame;
use std::sync::mpsc::channel;
use structs::{raster::RasterInfo, xarray::Xarray};

fn rusterize_rust(
    geometry: Vec<Geometry>,
    raster_info: &mut RasterInfo,
    pixel_fn: PixelFn,
    background: f64,
    threads: usize,
    df: Option<DataFrame>,
    field_name: Option<&str>,
    by_name: Option<&str>,
) -> (Array3<f64>, Vec<String>) {
    // validate geometries
    let (good_geom, df) = validate_geometries(geometry, df, raster_info);

    // extract `field` and `by`
    let casted: DataFrame;
    let (field, by) = match df {
        None => (
            // case 1: create a dummy `field`
            &Float64Chunked::from_vec(PlSmallStr::from("field_f64"), vec![1.0; good_geom.len()]),
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

            // collect casted dataframe
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
            let n_groups = groups.len();
            raster = raster_info.build_raster(n_groups, background);

            // notetaker for band order
            let mut order = vec![String::new(); n_groups];
            
            // init local thread pool
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
    
            pool.install(|| {
                // parallel iterator along bands, zipped with the corresponding groups
                raster
                    .outer_iter_mut()
                    .into_par_iter()
                    .zip(groups.into_idx().into_par_iter().enumerate())
                    .for_each_with(
                        sender,
                        |sendr, (mut band, (enum_idx, (group_idx, idxs)))| {
                            // send band names to receiver
                            if let Some(name) = by.get(group_idx as usize) {
                                sendr.send((enum_idx, name.to_string())).unwrap();
                            }

                            // rasterize polygons
                            for &i in idxs.iter() {
                                if let (Some(fv), Some(geom)) =
                                    (field.get(i as usize), good_geom.get(i as usize))
                                {
                                    rasterize_polygon(
                                        raster_info,
                                        geom,
                                        &fv,
                                        &mut band,
                                        &pixel_fn,
                                        &background,
                                    );
                                }
                            }
                        },
                    )  
                });

            // collect band names from the receiver and reorder
            for (enum_idx, name) in receiver.iter() {
                order[enum_idx] = name;
            }
            band_names = order;
        }
        None => {
            // singleband raster
            raster = raster_info.build_raster(1, background);

            // rasterize polygons
            field
                .into_iter()
                .zip(good_geom)
                .for_each(|(field_value, geom)| {
                    if let Some(fv) = field_value {
                        // process only non-empty field values
                        rasterize_polygon(
                            raster_info,
                            &geom,
                            &fv,
                            &mut raster.index_axis_mut(Axis(0), 0),
                            &pixel_fn,
                            &background,
                        )
                    }
                });
        }
    }

    (raster, band_names)
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pygeometry: &Bound<'py, PyAny>,
    pyinfo: &Bound<'py, PyAny>,
    pypixel_fn: &str,
    pythreads: &Bound<'py, PyAny>,
    pydf: Option<PyDataFrame>,
    pyfield: Option<&str>,
    pyby: Option<&str>,
    pybackground: Option<&Bound<'py, PyAny>>,
) -> PyResult<Xarray<'py>> {
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
    let mut raster_info = RasterInfo::from(pyinfo);

    // extract function arguments
    let pixel_fn = set_pixel_function(pypixel_fn);
    let background = pybackground
        .and_then(|inner| inner.extract::<f64>().ok())
        .unwrap_or(f64::NAN);

    // rusterize
    let (ret, band_names) = rusterize_rust(
        geometry,
        &mut raster_info,
        pixel_fn,
        background,
        threads,
        df,
        pyfield,
        pyby,
    );

    // construct coordinates
    let (y_coords, x_coords) = raster_info.make_coordinates(py);

    // to python
    let pyret = ret.into_pyarray_bound(py);
    let pybands = PyList::new_bound(py, band_names);
    let pydims = PyList::new_bound(py, vec!["bands", "y", "x"]);

    // build xarray dictionary
    let xarray = Xarray::build_xarray(pyret, pydims, x_coords, y_coords, pybands);
    Ok(xarray)
}

#[pymodule]
fn rusterize(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
