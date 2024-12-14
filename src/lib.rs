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
use polars::export::arrow::array::BooleanArray;
use polars::prelude::*;
use py_geo_interface::wrappers::f64::AsGeometryVec;
use pyo3::{
    prelude::*,
    types::{PyAny, PyString},
};
use pyo3_polars::PyDataFrame;
use structs::raster::Raster;
use wkt::ToWkt;

fn plexpr(
    geometry: &Vec<Geometry>,
    raster: &mut Array3<f64>,
    ras_info: &Raster,
    pixel_fn: &PixelFn,
    field: &String,
    band_idx: &mut usize,
    by: &String,
) -> Expr {
    as_struct(vec![col(field.clone()), col("gidx")]).map(
        |s| {
            let sc = s.struct_()?;
            let gidx_sc = sc.field_by_name("gidx")?.u64()?;
            let field_sc = sc.field_by_name(field.as_str())?.f64()?;
            // apply function to each field-idx pair
            let _ = field_sc
                .into_iter()
                .zip(gidx_sc.into_iter())
                .map(|(field_value, idx)| match field_value {
                    Some(field_value) => {
                        // field_value not empty
                        let uidx = idx.unwrap() as usize;
                        rasterize_polygon(
                            &ras_info,
                            &geometry[uidx],
                            &field_value,
                            &raster.index_axis_mut(Axis(0), *band_idx),
                            &pixel_fn,
                        )
                    }
                    _ => false,
                })
                .collect::<Vec<bool>>();
            // band index for group by
            if !by.is_empty() {
                *band_idx += 1
            }
            Ok(Some(s))
        },
        // define output type
        GetOutput::from_type(DataType::Struct(vec![
            Field::new(PlSmallStr::from(field), DataType::Float64),
            Field::new(PlSmallStr::from("gidx"), DataType::UInt64),
        ])),
    )
}

fn rusterize_rust(
    df: DataFrame,
    geometry: Vec<Geometry>,
    ras_info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    field: String,
    by: String,
) -> Array3<f64> {
    // polars does not allow Geometry, so add index for later query
    let range: Vec<u64> = (0..=geometry.len() as u64).collect();
    let geom_idx = Series::from_vec(PlSmallStr::from_str("gidx"), range);
    let df_lazy = df.lazy().with_column(geom_idx.lit());

    // index for building multiband raster
    let mut band_idx: usize = 0;

    if by.is_empty() {
        // build raster
        let mut raster = ras_info.build_raster(1);

        // rasterize each polygon iteratively
        df_lazy
            .with_columns([plexpr(
                &geometry,
                &mut raster,
                &ras_info,
                &pixel_fn,
                &field,
                &mut band_idx,
                &by,
            )])
            .collect()
            .map_err(PolarsError::from)
            .unwrap();

        // return
        raster
    } else {
        // determine groups
        let groups = df_lazy.group_by([col(by.clone())]);
        let bands = groups
            .clone()
            .agg([])
            .collect()
            .map(|df| df.height())
            .unwrap();

        // build raster
        let mut raster = ras_info.build_raster(bands);

        // rasterize each polygon iteratively by group
        groups
            .agg([plexpr(
                &geometry,
                &mut raster,
                &ras_info,
                &pixel_fn,
                &field,
                &mut band_idx,
                &by,
            )])
            .collect()
            .map_err(PolarsError::from)
            .unwrap();

        // return
        raster
    }
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pydf: PyDataFrame,
    pygeometry: PyAny,
    pyinfo: PyAny,
    pypixel_fn: PyString,
    pybackground: PyAny,
    pyfield: Option<PyString>,
    pyby: Option<PyString>,
) -> PyResult<&'py PyArray3<f64>> {
    // extract dataframe
    let df = pydf.into()?;

    // extract geometries
    let geometry = pygeometry.as_geometry_vec()?.0;

    // extract raster information
    let raster_info = Raster::from(&pyinfo);

    // extract function arguments
    let fun = pypixel_fn.to_str()?;
    let pixel_fn = set_pixel_function(fun).unwrap();
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
    let output = py.allow_threads(|| {
        rusterize_rust(df, geometry, raster_info, pixel_fn, background, field, by)
    });
    let ret = output.to_pyarray(py);
    Ok(ret)
}

fn rusterize(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
