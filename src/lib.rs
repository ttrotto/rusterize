#![feature(extract_if)]

mod allocator;
mod structs {
    pub mod edge;
    pub mod raster;
    pub mod xarray;
}
mod edge_collection;
mod geo_validate;
mod pixel_functions;
mod rasterize;

use crate::geo_validate::validate_geometries;
use crate::pixel_functions::{set_pixel_function, PixelFn};
use crate::rasterize::rasterize;
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
use structs::{raster::RasterInfo, xarray::Xarray};

fn rusterize_rust(
    geometry: Vec<Geometry>,
    raster_info: &mut RasterInfo,
    pixel_fn: PixelFn,
    background: f64,
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
            // get groups
            let groups = by.group_tuples(true, false).expect("No groups found!");
            let n_groups = groups.len();
            let group_idx = groups.into_idx();

            // multiband raster
            raster = raster_info.build_raster(n_groups, background);

            // dynamically set number of threads
            let cpus = num_cpus::get();
            let num_threads = n_groups.min(cpus / 2);

            // init thread pool
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap();

            // parallel iterator along bands, zipped with corresponding groups
            pool.install(|| {
                raster
                    .outer_iter_mut()
                    .into_par_iter()
                    .zip(group_idx.into_par_iter())
                    .map(|(mut band, (group_idx, idxs))| {
                        // rasterize polygons
                        for &i in idxs.iter() {
                            if let (Some(fv), Some(geom)) =
                                (field.get(i as usize), good_geom.get(i as usize))
                            {
                                // process only non-empty field values
                                rasterize(
                                    raster_info,
                                    geom,
                                    &fv,
                                    &mut band,
                                    &pixel_fn,
                                    &background,
                                );
                            }
                        }
                        // band name
                        by.get(group_idx as usize).unwrap().to_string()
                    })
                    .collect_into_vec(&mut band_names)
            });
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
                        rasterize(
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
#[allow(clippy::too_many_arguments)]
fn rusterize_py<'py>(
    py: Python<'py>,
    pygeometry: &Bound<'py, PyAny>,
    pyinfo: &Bound<'py, PyAny>,
    pypixel_fn: &str,
    pydf: Option<PyDataFrame>,
    pyfield: Option<&str>,
    pyby: Option<&str>,
    pybackground: Option<&Bound<'py, PyAny>>,
) -> PyResult<Xarray<'py>> {
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
